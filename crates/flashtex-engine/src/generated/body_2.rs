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
    /// \TeX\ is occasionally supposed to print diagnostic information that
    /// goes only into the transcript file, unless `tracing_online` is positive.
    /// Here are two routines that adjust the destination of print commands:
    // §245
    pub fn begin_diagnostic(&mut self) {
        self.old_setting = self.selector;
        if ((self.eqtb[((5292i32) - 1) as usize].int() <= 0i32) && (self.selector == 19i32)) {
            {
                self.selector = (self.selector).wrapping_sub(1i32);
                if (self.history == 0i32) {
                    self.history = 1i32;
                }
            }
        }
    }

    /// \TeX\ is occasionally supposed to print diagnostic information that
    /// goes only into the transcript file, unless `tracing_online` is positive.
    /// Here are two routines that adjust the destination of print commands:
    // §245
    pub fn end_diagnostic(&mut self, mut blank_line: bool) {
        self.print_nl(338i32);
        if blank_line {
            self.print_ln();
        }
        self.selector = self.old_setting;
    }

    /// The final region of `eqtb` contains the dimension parameters defined
    /// here, and the 256 \.{\\dimen} registers.
    // §247
    pub fn print_length_param(&mut self, mut n: i32) {
        match n {
            0 => {
                self.print_esc(478i32);
            }
            1 => {
                self.print_esc(479i32);
            }
            2 => {
                self.print_esc(480i32);
            }
            3 => {
                self.print_esc(481i32);
            }
            4 => {
                self.print_esc(482i32);
            }
            5 => {
                self.print_esc(483i32);
            }
            6 => {
                self.print_esc(484i32);
            }
            7 => {
                self.print_esc(485i32);
            }
            8 => {
                self.print_esc(486i32);
            }
            9 => {
                self.print_esc(487i32);
            }
            10 => {
                self.print_esc(488i32);
            }
            11 => {
                self.print_esc(489i32);
            }
            12 => {
                self.print_esc(490i32);
            }
            13 => {
                self.print_esc(491i32);
            }
            14 => {
                self.print_esc(492i32);
            }
            15 => {
                self.print_esc(493i32);
            }
            16 => {
                self.print_esc(494i32);
            }
            17 => {
                self.print_esc(495i32);
            }
            18 => {
                self.print_esc(496i32);
            }
            19 => {
                self.print_esc(497i32);
            }
            20 => {
                self.print_esc(498i32);
            }
            _ => {
                self.print(499i32);
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
    // §298
    pub fn print_cmd_chr(&mut self, mut cmd: quarterword, mut chr_code: halfword) {
        match cmd {
            1 => {
                {
                    self.print(557i32);
                    self.print(chr_code);
                }
            }
            2 => {
                {
                    self.print(558i32);
                    self.print(chr_code);
                }
            }
            3 => {
                {
                    self.print(559i32);
                    self.print(chr_code);
                }
            }
            6 => {
                {
                    self.print(560i32);
                    self.print(chr_code);
                }
            }
            7 => {
                {
                    self.print(561i32);
                    self.print(chr_code);
                }
            }
            8 => {
                {
                    self.print(562i32);
                    self.print(chr_code);
                }
            }
            9 => {
                self.print(563i32);
            }
            10 => {
                {
                    self.print(564i32);
                    self.print(chr_code);
                }
            }
            11 => {
                {
                    self.print(565i32);
                    self.print(chr_code);
                }
            }
            12 => {
                {
                    self.print(566i32);
                    self.print(chr_code);
                }
            }
            75 | 76 => {
                // §227
                if (chr_code < 2900i32) {
                    self.print_skip_param((chr_code).wrapping_sub(2882i32));
                } else {
                    if (chr_code < 3156i32) {
                        {
                            self.print_esc(395i32);
                            self.print_int((chr_code).wrapping_sub(2900i32));
                        }
                    } else {
                        {
                            self.print_esc(396i32);
                            self.print_int((chr_code).wrapping_sub(3156i32));
                        }
                    }
                }
            }
            72 => {
                // §231
                if (chr_code >= 3422i32) {
                    {
                        self.print_esc(407i32);
                        self.print_int((chr_code).wrapping_sub(3422i32));
                    }
                } else {
                    match chr_code {
                        3413 => {
                            self.print_esc(398i32);
                        }
                        3414 => {
                            self.print_esc(399i32);
                        }
                        3415 => {
                            self.print_esc(400i32);
                        }
                        3416 => {
                            self.print_esc(401i32);
                        }
                        3417 => {
                            self.print_esc(402i32);
                        }
                        3418 => {
                            self.print_esc(403i32);
                        }
                        3419 => {
                            self.print_esc(404i32);
                        }
                        3420 => {
                            self.print_esc(405i32);
                        }
                        _ => {
                            self.print_esc(406i32);
                        }
                    }
                }
            }
            73 => {
                // §239
                if (chr_code < 5318i32) {
                    self.print_param((chr_code).wrapping_sub(5263i32));
                } else {
                    {
                        self.print_esc(476i32);
                        self.print_int((chr_code).wrapping_sub(5318i32));
                    }
                }
            }
            74 => {
                // §249
                if (chr_code < 5851i32) {
                    self.print_length_param((chr_code).wrapping_sub(5830i32));
                } else {
                    {
                        self.print_esc(500i32);
                        self.print_int((chr_code).wrapping_sub(5851i32));
                    }
                }
            }
            45 => {
                // §266
                self.print_esc(508i32);
            }
            90 => {
                self.print_esc(509i32);
            }
            40 => {
                self.print_esc(510i32);
            }
            41 => {
                self.print_esc(511i32);
            }
            77 => {
                self.print_esc(519i32);
            }
            61 => {
                self.print_esc(512i32);
            }
            42 => {
                self.print_esc(531i32);
            }
            16 => {
                self.print_esc(513i32);
            }
            107 => {
                self.print_esc(504i32);
            }
            88 => {
                self.print_esc(518i32);
            }
            15 => {
                self.print_esc(514i32);
            }
            92 => {
                self.print_esc(515i32);
            }
            67 => {
                self.print_esc(505i32);
            }
            62 => {
                self.print_esc(516i32);
            }
            64 => {
                self.print_esc(32i32);
            }
            102 => {
                self.print_esc(517i32);
            }
            32 => {
                self.print_esc(520i32);
            }
            36 => {
                self.print_esc(521i32);
            }
            39 => {
                self.print_esc(522i32);
            }
            37 => {
                self.print_esc(330i32);
            }
            44 => {
                self.print_esc(47i32);
            }
            18 => {
                self.print_esc(351i32);
            }
            46 => {
                self.print_esc(523i32);
            }
            17 => {
                self.print_esc(524i32);
            }
            54 => {
                self.print_esc(525i32);
            }
            91 => {
                self.print_esc(526i32);
            }
            34 => {
                self.print_esc(527i32);
            }
            65 => {
                self.print_esc(528i32);
            }
            103 => {
                self.print_esc(529i32);
            }
            55 => {
                self.print_esc(335i32);
            }
            63 => {
                self.print_esc(530i32);
            }
            66 => {
                self.print_esc(533i32);
            }
            96 => {
                self.print_esc(534i32);
            }
            0 => {
                self.print_esc(535i32);
            }
            98 => {
                self.print_esc(536i32);
            }
            80 => {
                self.print_esc(532i32);
            }
            84 => {
                self.print_esc(408i32);
            }
            109 => {
                self.print_esc(537i32);
            }
            71 => {
                self.print_esc(407i32);
            }
            38 => {
                self.print_esc(352i32);
            }
            33 => {
                self.print_esc(538i32);
            }
            56 => {
                self.print_esc(539i32);
            }
            35 => {
                self.print_esc(540i32);
            }
            13 => {
                // §335
                self.print_esc(597i32);
            }
            104 => {
                // §377
                if (chr_code == 0i32) {
                    self.print_esc(629i32);
                } else {
                    self.print_esc(630i32);
                }
            }
            110 => {
                // §385
                match chr_code {
                    1 => {
                        self.print_esc(632i32);
                    }
                    2 => {
                        self.print_esc(633i32);
                    }
                    3 => {
                        self.print_esc(634i32);
                    }
                    4 => {
                        self.print_esc(635i32);
                    }
                    _ => {
                        self.print_esc(631i32);
                    }
                }
            }
            89 => {
                // §412
                if (chr_code == 0i32) {
                    self.print_esc(476i32);
                } else {
                    if (chr_code == 1i32) {
                        self.print_esc(500i32);
                    } else {
                        if (chr_code == 2i32) {
                            self.print_esc(395i32);
                        } else {
                            self.print_esc(396i32);
                        }
                    }
                }
            }
            79 => {
                // §417
                if (chr_code == 1i32) {
                    self.print_esc(669i32);
                } else {
                    self.print_esc(668i32);
                }
            }
            82 => {
                if (chr_code == 0i32) {
                    self.print_esc(670i32);
                } else {
                    self.print_esc(671i32);
                }
            }
            83 => {
                if (chr_code == 1i32) {
                    self.print_esc(672i32);
                } else {
                    if (chr_code == 3i32) {
                        self.print_esc(673i32);
                    } else {
                        self.print_esc(674i32);
                    }
                }
            }
            70 => {
                match chr_code {
                    0 => {
                        self.print_esc(675i32);
                    }
                    1 => {
                        self.print_esc(676i32);
                    }
                    2 => {
                        self.print_esc(677i32);
                    }
                    3 => {
                        self.print_esc(678i32);
                    }
                    _ => {
                        self.print_esc(679i32);
                    }
                }
            }
            108 => {
                // §469
                match chr_code {
                    0 => {
                        self.print_esc(735i32);
                    }
                    1 => {
                        self.print_esc(736i32);
                    }
                    2 => {
                        self.print_esc(737i32);
                    }
                    3 => {
                        self.print_esc(738i32);
                    }
                    4 => {
                        self.print_esc(739i32);
                    }
                    _ => {
                        self.print_esc(740i32);
                    }
                }
            }
            105 => {
                // §488
                match chr_code {
                    1 => {
                        self.print_esc(758i32);
                    }
                    2 => {
                        self.print_esc(759i32);
                    }
                    3 => {
                        self.print_esc(760i32);
                    }
                    4 => {
                        self.print_esc(761i32);
                    }
                    5 => {
                        self.print_esc(762i32);
                    }
                    6 => {
                        self.print_esc(763i32);
                    }
                    7 => {
                        self.print_esc(764i32);
                    }
                    8 => {
                        self.print_esc(765i32);
                    }
                    9 => {
                        self.print_esc(766i32);
                    }
                    10 => {
                        self.print_esc(767i32);
                    }
                    11 => {
                        self.print_esc(768i32);
                    }
                    12 => {
                        self.print_esc(769i32);
                    }
                    13 => {
                        self.print_esc(770i32);
                    }
                    14 => {
                        self.print_esc(771i32);
                    }
                    15 => {
                        self.print_esc(772i32);
                    }
                    16 => {
                        self.print_esc(773i32);
                    }
                    _ => {
                        self.print_esc(757i32);
                    }
                }
            }
            106 => {
                // §492
                if (chr_code == 2i32) {
                    self.print_esc(774i32);
                } else {
                    if (chr_code == 4i32) {
                        self.print_esc(775i32);
                    } else {
                        self.print_esc(776i32);
                    }
                }
            }
            4 => {
                // §781
                if (chr_code == 256i32) {
                    self.print_esc(898i32);
                } else {
                    {
                        self.print(902i32);
                        self.print(chr_code);
                    }
                }
            }
            5 => {
                if (chr_code == 257i32) {
                    self.print_esc(899i32);
                } else {
                    self.print_esc(900i32);
                }
            }
            81 => {
                // §984
                match chr_code {
                    0 => {
                        self.print_esc(970i32);
                    }
                    1 => {
                        self.print_esc(971i32);
                    }
                    2 => {
                        self.print_esc(972i32);
                    }
                    3 => {
                        self.print_esc(973i32);
                    }
                    4 => {
                        self.print_esc(974i32);
                    }
                    5 => {
                        self.print_esc(975i32);
                    }
                    6 => {
                        self.print_esc(976i32);
                    }
                    _ => {
                        self.print_esc(977i32);
                    }
                }
            }
            14 => {
                // §1053
                if (chr_code == 1i32) {
                    self.print_esc(1026i32);
                } else {
                    self.print_esc(1025i32);
                }
            }
            26 => {
                // §1059
                match chr_code {
                    4 => {
                        self.print_esc(1027i32);
                    }
                    0 => {
                        self.print_esc(1028i32);
                    }
                    1 => {
                        self.print_esc(1029i32);
                    }
                    2 => {
                        self.print_esc(1030i32);
                    }
                    _ => {
                        self.print_esc(1031i32);
                    }
                }
            }
            27 => {
                match chr_code {
                    4 => {
                        self.print_esc(1032i32);
                    }
                    0 => {
                        self.print_esc(1033i32);
                    }
                    1 => {
                        self.print_esc(1034i32);
                    }
                    2 => {
                        self.print_esc(1035i32);
                    }
                    _ => {
                        self.print_esc(1036i32);
                    }
                }
            }
            28 => {
                self.print_esc(336i32);
            }
            29 => {
                self.print_esc(340i32);
            }
            30 => {
                self.print_esc(342i32);
            }
            21 => {
                // §1072
                if (chr_code == 1i32) {
                    self.print_esc(1054i32);
                } else {
                    self.print_esc(1055i32);
                }
            }
            22 => {
                if (chr_code == 1i32) {
                    self.print_esc(1056i32);
                } else {
                    self.print_esc(1057i32);
                }
            }
            20 => {
                match chr_code {
                    0 => {
                        self.print_esc(409i32);
                    }
                    1 => {
                        self.print_esc(1058i32);
                    }
                    2 => {
                        self.print_esc(1059i32);
                    }
                    3 => {
                        self.print_esc(965i32);
                    }
                    4 => {
                        self.print_esc(1060i32);
                    }
                    5 => {
                        self.print_esc(967i32);
                    }
                    _ => {
                        self.print_esc(1061i32);
                    }
                }
            }
            31 => {
                if (chr_code == 100i32) {
                    self.print_esc(1063i32);
                } else {
                    if (chr_code == 101i32) {
                        self.print_esc(1064i32);
                    } else {
                        if (chr_code == 102i32) {
                            self.print_esc(1065i32);
                        } else {
                            self.print_esc(1062i32);
                        }
                    }
                }
            }
            43 => {
                // §1089
                if (chr_code == 0i32) {
                    self.print_esc(1081i32);
                } else {
                    self.print_esc(1080i32);
                }
            }
            25 => {
                // §1108
                if (chr_code == 10i32) {
                    self.print_esc(1092i32);
                } else {
                    if (chr_code == 11i32) {
                        self.print_esc(1091i32);
                    } else {
                        self.print_esc(1090i32);
                    }
                }
            }
            23 => {
                if (chr_code == 1i32) {
                    self.print_esc(1094i32);
                } else {
                    self.print_esc(1093i32);
                }
            }
            24 => {
                if (chr_code == 1i32) {
                    self.print_esc(1096i32);
                } else {
                    self.print_esc(1095i32);
                }
            }
            47 => {
                // §1115
                if (chr_code == 1i32) {
                    self.print_esc(45i32);
                } else {
                    self.print_esc(349i32);
                }
            }
            48 => {
                // §1143
                if (chr_code == 1i32) {
                    self.print_esc(1128i32);
                } else {
                    self.print_esc(1127i32);
                }
            }
            50 => {
                // §1157
                match chr_code {
                    16 => {
                        self.print_esc(866i32);
                    }
                    17 => {
                        self.print_esc(867i32);
                    }
                    18 => {
                        self.print_esc(868i32);
                    }
                    19 => {
                        self.print_esc(869i32);
                    }
                    20 => {
                        self.print_esc(870i32);
                    }
                    21 => {
                        self.print_esc(871i32);
                    }
                    22 => {
                        self.print_esc(872i32);
                    }
                    23 => {
                        self.print_esc(873i32);
                    }
                    26 => {
                        self.print_esc(875i32);
                    }
                    _ => {
                        self.print_esc(874i32);
                    }
                }
            }
            51 => {
                if (chr_code == 1i32) {
                    self.print_esc(878i32);
                } else {
                    if (chr_code == 2i32) {
                        self.print_esc(879i32);
                    } else {
                        self.print_esc(1129i32);
                    }
                }
            }
            53 => {
                // §1170
                self.print_style(chr_code);
            }
            52 => {
                // §1179
                match chr_code {
                    1 => {
                        self.print_esc(1148i32);
                    }
                    2 => {
                        self.print_esc(1149i32);
                    }
                    3 => {
                        self.print_esc(1150i32);
                    }
                    4 => {
                        self.print_esc(1151i32);
                    }
                    5 => {
                        self.print_esc(1152i32);
                    }
                    _ => {
                        self.print_esc(1147i32);
                    }
                }
            }
            49 => {
                // §1189
                if (chr_code == 30i32) {
                    self.print_esc(876i32);
                } else {
                    self.print_esc(877i32);
                }
            }
            93 => {
                // §1209
                if (chr_code == 1i32) {
                    self.print_esc(1171i32);
                } else {
                    if (chr_code == 2i32) {
                        self.print_esc(1172i32);
                    } else {
                        self.print_esc(1173i32);
                    }
                }
            }
            97 => {
                if (chr_code == 0i32) {
                    self.print_esc(1174i32);
                } else {
                    if (chr_code == 1i32) {
                        self.print_esc(1175i32);
                    } else {
                        if (chr_code == 2i32) {
                            self.print_esc(1176i32);
                        } else {
                            self.print_esc(1177i32);
                        }
                    }
                }
            }
            94 => {
                // §1220
                if (chr_code != 0i32) {
                    self.print_esc(1192i32);
                } else {
                    self.print_esc(1191i32);
                }
            }
            95 => {
                // §1223
                match chr_code {
                    0 => {
                        self.print_esc(1193i32);
                    }
                    1 => {
                        self.print_esc(1194i32);
                    }
                    2 => {
                        self.print_esc(1195i32);
                    }
                    3 => {
                        self.print_esc(1196i32);
                    }
                    4 => {
                        self.print_esc(1197i32);
                    }
                    5 => {
                        self.print_esc(1198i32);
                    }
                    _ => {
                        self.print_esc(1199i32);
                    }
                }
            }
            68 => {
                {
                    self.print_esc(513i32);
                    self.print_hex(chr_code);
                }
            }
            69 => {
                {
                    self.print_esc(524i32);
                    self.print_hex(chr_code);
                }
            }
            85 => {
                // §1231
                if (chr_code == 3983i32) {
                    self.print_esc(415i32);
                } else {
                    if (chr_code == 5007i32) {
                        self.print_esc(419i32);
                    } else {
                        if (chr_code == 4239i32) {
                            self.print_esc(416i32);
                        } else {
                            if (chr_code == 4495i32) {
                                self.print_esc(417i32);
                            } else {
                                if (chr_code == 4751i32) {
                                    self.print_esc(418i32);
                                } else {
                                    self.print_esc(477i32);
                                }
                            }
                        }
                    }
                }
            }
            86 => {
                self.print_size((chr_code).wrapping_sub(3935i32));
            }
            99 => {
                // §1251
                if (chr_code == 1i32) {
                    self.print_esc(953i32);
                } else {
                    self.print_esc(941i32);
                }
            }
            78 => {
                // §1255
                if (chr_code == 0i32) {
                    self.print_esc(1217i32);
                } else {
                    self.print_esc(1218i32);
                }
            }
            87 => {
                // §1261
                {
                    self.print(1226i32);
                    self.slow_print(self.font_name[(chr_code) as usize]);
                    if (self.font_size[(chr_code) as usize] != self.font_dsize[(chr_code) as usize]) {
                        {
                            self.print(741i32);
                            self.print_scaled(self.font_size[(chr_code) as usize]);
                            self.print(397i32);
                        }
                    }
                }
            }
            100 => {
                // §1263
                match chr_code {
                    0 => {
                        self.print_esc(274i32);
                    }
                    1 => {
                        self.print_esc(275i32);
                    }
                    2 => {
                        self.print_esc(276i32);
                    }
                    _ => {
                        self.print_esc(1227i32);
                    }
                }
            }
            60 => {
                // §1273
                if (chr_code == 0i32) {
                    self.print_esc(1229i32);
                } else {
                    self.print_esc(1228i32);
                }
            }
            58 => {
                // §1278
                if (chr_code == 0i32) {
                    self.print_esc(1230i32);
                } else {
                    self.print_esc(1231i32);
                }
            }
            57 => {
                // §1287
                if (chr_code == 4239i32) {
                    self.print_esc(1237i32);
                } else {
                    self.print_esc(1238i32);
                }
            }
            19 => {
                // §1292
                match chr_code {
                    1 => {
                        self.print_esc(1240i32);
                    }
                    2 => {
                        self.print_esc(1241i32);
                    }
                    3 => {
                        self.print_esc(1242i32);
                    }
                    _ => {
                        self.print_esc(1239i32);
                    }
                }
            }
            101 => {
                // §1295
                self.print(1249i32);
            }
            111 => {
                self.print(1250i32);
            }
            112 => {
                self.print_esc(1251i32);
            }
            113 => {
                self.print_esc(1252i32);
            }
            114 => {
                {
                    self.print_esc(1171i32);
                    self.print_esc(1252i32);
                }
            }
            115 => {
                self.print_esc(1253i32);
            }
            59 => {
                // §1346
                match chr_code {
                    0 => {
                        self.print_esc(1285i32);
                    }
                    1 => {
                        self.print_esc(594i32);
                    }
                    2 => {
                        self.print_esc(1286i32);
                    }
                    3 => {
                        self.print_esc(1287i32);
                    }
                    4 => {
                        self.print_esc(1288i32);
                    }
                    5 => {
                        self.print_esc(1289i32);
                    }
                    _ => {
                        self.print(1290i32);
                    }
                }
            }
            _ => {
                // §298
                self.print(567i32);
            }
        }
    }

    /// Here is a procedure that displays the contents of `eqtb[n]`
    /// symbolically.
    // §252
    pub fn show_eqtb(&mut self, mut n: halfword) {
        if (n < 1i32) {
            self.print_char(63i32);
        } else {
            if (n < 2882i32) {
                // §223
                {
                    self.sprint_cs(n);
                    self.print_char(61i32);
                    self.print_cmd_chr(self.eqtb[((n) - 1) as usize].hh().b0(), self.eqtb[((n) - 1) as usize].hh().rh());
                    if (self.eqtb[((n) - 1) as usize].hh().b0() >= 111i32) {
                        {
                            self.print_char(58i32);
                            self.show_token_list(self.mem[(self.eqtb[((n) - 1) as usize].hh().rh()) as usize].hh().rh(), 0i32, 32i32);
                        }
                    }
                }
            } else {
                // §252
                if (n < 3412i32) {
                    // §229
                    if (n < 2900i32) {
                        {
                            self.print_skip_param((n).wrapping_sub(2882i32));
                            self.print_char(61i32);
                            if (n < 2897i32) {
                                self.print_spec(self.eqtb[((n) - 1) as usize].hh().rh(), 397i32);
                            } else {
                                self.print_spec(self.eqtb[((n) - 1) as usize].hh().rh(), 337i32);
                            }
                        }
                    } else {
                        if (n < 3156i32) {
                            {
                                self.print_esc(395i32);
                                self.print_int((n).wrapping_sub(2900i32));
                                self.print_char(61i32);
                                self.print_spec(self.eqtb[((n) - 1) as usize].hh().rh(), 397i32);
                            }
                        } else {
                            {
                                self.print_esc(396i32);
                                self.print_int((n).wrapping_sub(3156i32));
                                self.print_char(61i32);
                                self.print_spec(self.eqtb[((n) - 1) as usize].hh().rh(), 337i32);
                            }
                        }
                    }
                } else {
                    // §252
                    if (n < 5263i32) {
                        // §233
                        if (n == 3412i32) {
                            {
                                self.print_esc(408i32);
                                self.print_char(61i32);
                                if (self.eqtb[((3412i32) - 1) as usize].hh().rh() == 0i32) {
                                    self.print_char(48i32);
                                } else {
                                    self.print_int(self.mem[(self.eqtb[((3412i32) - 1) as usize].hh().rh()) as usize].hh().lh());
                                }
                            }
                        } else {
                            if (n < 3422i32) {
                                {
                                    self.print_cmd_chr(72i32, n);
                                    self.print_char(61i32);
                                    if (self.eqtb[((n) - 1) as usize].hh().rh() != 0i32) {
                                        self.show_token_list(self.mem[(self.eqtb[((n) - 1) as usize].hh().rh()) as usize].hh().rh(), 0i32, 32i32);
                                    }
                                }
                            } else {
                                if (n < 3678i32) {
                                    {
                                        self.print_esc(407i32);
                                        self.print_int((n).wrapping_sub(3422i32));
                                        self.print_char(61i32);
                                        if (self.eqtb[((n) - 1) as usize].hh().rh() != 0i32) {
                                            self.show_token_list(self.mem[(self.eqtb[((n) - 1) as usize].hh().rh()) as usize].hh().rh(), 0i32, 32i32);
                                        }
                                    }
                                } else {
                                    if (n < 3934i32) {
                                        {
                                            self.print_esc(409i32);
                                            self.print_int((n).wrapping_sub(3678i32));
                                            self.print_char(61i32);
                                            if (self.eqtb[((n) - 1) as usize].hh().rh() == 0i32) {
                                                self.print(410i32);
                                            } else {
                                                {
                                                    self.depth_threshold = 0i32;
                                                    self.breadth_max = 1i32;
                                                    self.show_node_list(self.eqtb[((n) - 1) as usize].hh().rh());
                                                }
                                            }
                                        }
                                    } else {
                                        if (n < 3983i32) {
                                            // §234
                                            {
                                                if (n == 3934i32) {
                                                    self.print(411i32);
                                                } else {
                                                    if (n < 3951i32) {
                                                        {
                                                            self.print_esc(412i32);
                                                            self.print_int((n).wrapping_sub(3935i32));
                                                        }
                                                    } else {
                                                        if (n < 3967i32) {
                                                            {
                                                                self.print_esc(413i32);
                                                                self.print_int((n).wrapping_sub(3951i32));
                                                            }
                                                        } else {
                                                            {
                                                                self.print_esc(414i32);
                                                                self.print_int((n).wrapping_sub(3967i32));
                                                            }
                                                        }
                                                    }
                                                }
                                                self.print_char(61i32);
                                                self.print_esc(self.hash[(((2624i32).wrapping_add(self.eqtb[((n) - 1) as usize].hh().rh())) - 514) as usize].rh());
                                            }
                                        } else {
                                            // §235
                                            if (n < 5007i32) {
                                                {
                                                    if (n < 4239i32) {
                                                        {
                                                            self.print_esc(415i32);
                                                            self.print_int((n).wrapping_sub(3983i32));
                                                        }
                                                    } else {
                                                        if (n < 4495i32) {
                                                            {
                                                                self.print_esc(416i32);
                                                                self.print_int((n).wrapping_sub(4239i32));
                                                            }
                                                        } else {
                                                            if (n < 4751i32) {
                                                                {
                                                                    self.print_esc(417i32);
                                                                    self.print_int((n).wrapping_sub(4495i32));
                                                                }
                                                            } else {
                                                                {
                                                                    self.print_esc(418i32);
                                                                    self.print_int((n).wrapping_sub(4751i32));
                                                                }
                                                            }
                                                        }
                                                    }
                                                    self.print_char(61i32);
                                                    self.print_int(self.eqtb[((n) - 1) as usize].hh().rh());
                                                }
                                            } else {
                                                {
                                                    self.print_esc(419i32);
                                                    self.print_int((n).wrapping_sub(5007i32));
                                                    self.print_char(61i32);
                                                    self.print_int((self.eqtb[((n) - 1) as usize].hh().rh()).wrapping_sub(0i32));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // §252
                        if (n < 5830i32) {
                            // §242
                            {
                                if (n < 5318i32) {
                                    self.print_param((n).wrapping_sub(5263i32));
                                } else {
                                    if (n < 5574i32) {
                                        {
                                            self.print_esc(476i32);
                                            self.print_int((n).wrapping_sub(5318i32));
                                        }
                                    } else {
                                        {
                                            self.print_esc(477i32);
                                            self.print_int((n).wrapping_sub(5574i32));
                                        }
                                    }
                                }
                                self.print_char(61i32);
                                self.print_int(self.eqtb[((n) - 1) as usize].int());
                            }
                        } else {
                            // §252
                            if (n <= 6106i32) {
                                // §251
                                {
                                    if (n < 5851i32) {
                                        self.print_length_param((n).wrapping_sub(5830i32));
                                    } else {
                                        {
                                            self.print_esc(500i32);
                                            self.print_int((n).wrapping_sub(5851i32));
                                        }
                                    }
                                    self.print_char(61i32);
                                    self.print_scaled(self.eqtb[((n) - 1) as usize].int());
                                    self.print(397i32);
                                }
                            } else {
                                // §252
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
    // §259
    pub fn id_lookup(&mut self, mut j: i32, mut l: i32) -> halfword {
        let mut id_lookup: halfword = 0;
        let mut h: i32 = 0; // §259
        let mut d: i32 = 0; // §259
        let mut p: halfword = 0; // §259
        let mut k: halfword = 0; // §259
        'l_found_f: {
            // §261
            h = self.buffer[(j) as usize];
            {
                let __for_end_3 = ((j).wrapping_add(l)).wrapping_sub(1i32);
                k = (j).wrapping_add(1i32);
                while k <= __for_end_3 {
                    {
                        h = ((h).wrapping_add(h)).wrapping_add(self.buffer[(k) as usize]);
                        while (h >= 1777i32) {
                            h = (h).wrapping_sub(1777i32);
                        }
                    }
                    k = k.wrapping_add(1);
                }
            }
            // §259
            p = (h).wrapping_add(514i32);
            while true {
                {
                    if (self.hash[((p) - 514) as usize].rh() > 0i32) {
                        if ((self.str_start[((self.hash[((p) - 514) as usize].rh()).wrapping_add(1i32)) as usize]).wrapping_sub(self.str_start[(self.hash[((p) - 514) as usize].rh()) as usize]) == l) {
                            if self.str_eq_buf(self.hash[((p) - 514) as usize].rh(), j) {
                                break 'l_found_f;
                            }
                        }
                    }
                    if (self.hash[((p) - 514) as usize].lh() == 0i32) {
                        {
                            if self.no_new_control_sequence {
                                p = 2881i32;
                            } else {
                                // §260
                                {
                                    if (self.hash[((p) - 514) as usize].rh() > 0i32) {
                                        {
                                            loop {
                                                if (self.hash_used == 514i32) {
                                                    self.overflow(503i32, 2100i32);
                                                }
                                                self.hash_used = (self.hash_used).wrapping_sub(1i32);
                                                if (self.hash[((self.hash_used) - 514) as usize].rh() == 0i32) { break; }
                                            }
                                            { let __v85 = self.hash_used; self.hash[((p) - 514) as usize].set_lh(__v85); }
                                            p = self.hash_used;
                                        }
                                    }
                                    {
                                        if ((self.pool_ptr).wrapping_add(l) > pool_size) {
                                            self.overflow(257i32, (pool_size).wrapping_sub(self.init_pool_ptr));
                                        }
                                    }
                                    d = (self.pool_ptr).wrapping_sub(self.str_start[(self.str_ptr) as usize]);
                                    while (self.pool_ptr > self.str_start[(self.str_ptr) as usize]) {
                                        {
                                            self.pool_ptr = (self.pool_ptr).wrapping_sub(1i32);
                                            { let __ix86 = (self.pool_ptr).wrapping_add(l); let __v87 = self.str_pool[(self.pool_ptr) as usize]; self.str_pool[(__ix86) as usize] = __v87; }
                                        }
                                    }
                                    {
                                        let __for_end_9 = ((j).wrapping_add(l)).wrapping_sub(1i32);
                                        k = j;
                                        while k <= __for_end_9 {
                                            {
                                                { let __ix88 = self.pool_ptr; let __v89 = self.buffer[(k) as usize]; self.str_pool[(__ix88) as usize] = __v89; }
                                                self.pool_ptr = (self.pool_ptr).wrapping_add(1i32);
                                            }
                                            k = k.wrapping_add(1);
                                        }
                                    }
                                    { let __v90 = self.make_string(); self.hash[((p) - 514) as usize].set_rh(__v90); }
                                    self.pool_ptr = (self.pool_ptr).wrapping_add(d);
                                    self.cs_count = (self.cs_count).wrapping_add(1i32);
                                }
                            }
                            // §259
                            break 'l_found_f;
                        }
                    }
                    p = self.hash[((p) - 514) as usize].lh();
                }
            }
        }
        id_lookup = p;
        id_lookup
    }

    /// We need to put \TeX's ``primitive'' control sequences into the hash
    /// table, together with their command code (which will be the `eq_type`)
    /// and an operand (which will be the `equiv`). The `primitive` procedure
    /// does this, in a way that no \TeX\ user can. The global value `cur_val`
    /// contains the new `eqtb` pointer after `primitive` has acted.
    // §264
    pub fn primitive(&mut self, mut s: str_number, mut c: quarterword, mut o: halfword) {
        let mut k: pool_pointer = 0; // §264
        let mut j: small_number = 0; // §264
        let mut l: small_number = 0; // §264
        if (s < 256i32) {
            self.cur_val = (s).wrapping_add(257i32);
        } else {
            {
                k = self.str_start[(s) as usize];
                l = (self.str_start[((s).wrapping_add(1i32)) as usize]).wrapping_sub(k);
                {
                    let __for_end_4 = (l).wrapping_sub(1i32);
                    j = 0i32;
                    while j <= __for_end_4 {
                        { let __v91 = self.str_pool[((k).wrapping_add(j)) as usize]; self.buffer[(j) as usize] = __v91; }
                        j = j.wrapping_add(1);
                    }
                }
                self.cur_val = self.id_lookup(0i32, l);
                {
                    self.str_ptr = (self.str_ptr).wrapping_sub(1i32);
                    self.pool_ptr = self.str_start[(self.str_ptr) as usize];
                }
                { let __ix92 = self.cur_val; self.hash[((__ix92) - 514) as usize].set_rh(s); }
            }
        }
        { let __ix93 = self.cur_val; self.eqtb[((__ix93) - 1) as usize].set_hh_b1(1i32); }
        { let __ix94 = self.cur_val; self.eqtb[((__ix94) - 1) as usize].set_hh_b0(c); }
        { let __ix95 = self.cur_val; self.eqtb[((__ix95) - 1) as usize].set_hh_rh(o); }
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
    // §274
    pub fn new_save_level(&mut self, mut c: group_code) {
        if (self.save_ptr > self.max_save_stack) {
            {
                self.max_save_stack = self.save_ptr;
                if (self.max_save_stack > (save_size).wrapping_sub(6i32)) {
                    self.overflow(541i32, save_size);
                }
            }
        }
        { let __ix96 = self.save_ptr; self.save_stack[(__ix96) as usize].set_hh_b0(3i32); }
        { let __ix97 = self.save_ptr; let __v98 = self.cur_group; self.save_stack[(__ix97) as usize].set_hh_b1(__v98); }
        { let __ix99 = self.save_ptr; let __v100 = self.cur_boundary; self.save_stack[(__ix99) as usize].set_hh_rh(__v100); }
        if (self.cur_level == 255i32) {
            self.overflow(542i32, 255i32);
        }
        self.cur_boundary = self.save_ptr;
        self.cur_level = (self.cur_level).wrapping_add(1i32);
        self.save_ptr = (self.save_ptr).wrapping_add(1i32);
        self.cur_group = c;
    }

    /// Just before an entry of `eqtb` is changed, the following procedure should
    /// be called to update the other data structures properly. It is important
    /// to keep in mind that reference counts in `mem` include references from
    /// within `save_stack`, so these counts must be handled carefully.
    // §275
    pub fn eq_destroy(&mut self, mut w: memory_word) {
        let mut q: halfword = 0; // §275
        match w.hh().b0() {
            111 | 112 | 113 | 114 => {
                self.delete_token_ref(w.hh().rh());
            }
            117 => {
                self.delete_glue_ref(w.hh().rh());
            }
            118 => {
                {
                    q = w.hh().rh();
                    if (q != 0i32) {
                        self.free_node(q, ((self.mem[(q) as usize].hh().lh()).wrapping_add(self.mem[(q) as usize].hh().lh())).wrapping_add(1i32));
                    }
                }
            }
            119 => {
                self.flush_node_list(w.hh().rh());
            }
            _ => {
            }
        }
    }

    /// To save a value of `eqtb[p]` that was established at level `l`, we
    /// can use the following subroutine.
    // §276
    pub fn eq_save(&mut self, mut p: halfword, mut l: quarterword) {
        if (self.save_ptr > self.max_save_stack) {
            {
                self.max_save_stack = self.save_ptr;
                if (self.max_save_stack > (save_size).wrapping_sub(6i32)) {
                    self.overflow(541i32, save_size);
                }
            }
        }
        if (l == 0i32) {
            { let __ix101 = self.save_ptr; self.save_stack[(__ix101) as usize].set_hh_b0(1i32); }
        } else {
            {
                { let __ix102 = self.save_ptr; let __v103 = self.eqtb[((p) - 1) as usize]; self.save_stack[(__ix102) as usize] = __v103; }
                self.save_ptr = (self.save_ptr).wrapping_add(1i32);
                { let __ix104 = self.save_ptr; self.save_stack[(__ix104) as usize].set_hh_b0(0i32); }
            }
        }
        { let __ix105 = self.save_ptr; self.save_stack[(__ix105) as usize].set_hh_b1(l); }
        { let __ix106 = self.save_ptr; self.save_stack[(__ix106) as usize].set_hh_rh(p); }
        self.save_ptr = (self.save_ptr).wrapping_add(1i32);
    }

    /// The procedure `eq_define` defines an `eqtb` entry having specified
    /// `eq_type` and `equiv` fields, and saves the former value if appropriate.
    /// This procedure is used only for entries in the first four regions of `eqtb`,
    /// i.e., only for entries that have `eq_type` and `equiv` fields.
    /// After calling this routine, it is safe to put four more entries on
    /// `save_stack`, provided that there was room for four more entries before
    /// the call, since `eq_save` makes the necessary test.
    // §277
    pub fn eq_define(&mut self, mut p: halfword, mut t: quarterword, mut e: halfword) {
        if (self.eqtb[((p) - 1) as usize].hh().b1() == self.cur_level) {
            self.eq_destroy(self.eqtb[((p) - 1) as usize]);
        } else {
            if (self.cur_level > 1i32) {
                self.eq_save(p, self.eqtb[((p) - 1) as usize].hh().b1());
            }
        }
        { let __v107 = self.cur_level; self.eqtb[((p) - 1) as usize].set_hh_b1(__v107); }
        self.eqtb[((p) - 1) as usize].set_hh_b0(t);
        self.eqtb[((p) - 1) as usize].set_hh_rh(e);
    }

    /// The counterpart of `eq_define` for the remaining (fullword) positions in
    /// `eqtb` is called `eq_word_define`. Since `xeq_level[p]>=level_one` for all
    /// `p`, a ``restore_zero`' will never be used in this case.
    // §278
    pub fn eq_word_define(&mut self, mut p: halfword, mut w: i32) {
        if (self.xeq_level[((p) - 5263) as usize] != self.cur_level) {
            {
                self.eq_save(p, self.xeq_level[((p) - 5263) as usize]);
                { let __v108 = self.cur_level; self.xeq_level[((p) - 5263) as usize] = __v108; }
            }
        }
        self.eqtb[((p) - 1) as usize].set_int(w);
    }

    /// The `eq_define` and `eq_word_define` routines take care of local definitions.
    /// Global definitions are done in almost the same way, but there is no need
    /// to save old values, and the new value is associated with `level_one`.
    // §279
    pub fn geq_define(&mut self, mut p: halfword, mut t: quarterword, mut e: halfword) {
        self.eq_destroy(self.eqtb[((p) - 1) as usize]);
        self.eqtb[((p) - 1) as usize].set_hh_b1(1i32);
        self.eqtb[((p) - 1) as usize].set_hh_b0(t);
        self.eqtb[((p) - 1) as usize].set_hh_rh(e);
    }

    /// The `eq_define` and `eq_word_define` routines take care of local definitions.
    /// Global definitions are done in almost the same way, but there is no need
    /// to save old values, and the new value is associated with `level_one`.
    // §279
    pub fn geq_word_define(&mut self, mut p: halfword, mut w: i32) {
        self.eqtb[((p) - 1) as usize].set_int(w);
        self.xeq_level[((p) - 5263) as usize] = 1i32;
    }

    /// Subroutine `save_for_after` puts a token on the stack for save-keeping.
    // §280
    pub fn save_for_after(&mut self, mut t: halfword) {
        if (self.cur_level > 1i32) {
            {
                if (self.save_ptr > self.max_save_stack) {
                    {
                        self.max_save_stack = self.save_ptr;
                        if (self.max_save_stack > (save_size).wrapping_sub(6i32)) {
                            self.overflow(541i32, save_size);
                        }
                    }
                }
                { let __ix109 = self.save_ptr; self.save_stack[(__ix109) as usize].set_hh_b0(2i32); }
                { let __ix110 = self.save_ptr; self.save_stack[(__ix110) as usize].set_hh_b1(0i32); }
                { let __ix111 = self.save_ptr; self.save_stack[(__ix111) as usize].set_hh_rh(t); }
                self.save_ptr = (self.save_ptr).wrapping_add(1i32);
            }
        }
    }

    /// @<Declare the procedure called `restore_trace`
    // §284
    pub fn restore_trace(&mut self, mut p: halfword, mut s: str_number) {
        self.begin_diagnostic();
        self.print_char(123i32);
        self.print(s);
        self.print_char(32i32);
        self.show_eqtb(p);
        self.print_char(125i32);
        self.end_diagnostic(false);
    }

    /// The `unsave` routine goes the other way, taking items off of `save_stack`.
    /// This routine takes care of restoration when a level ends; everything
    /// belonging to the topmost group is cleared off of the save stack.
    // §281
    pub fn unsave(&mut self) {
        let mut p: halfword = 0; // §281
        let mut l: quarterword = 0; // §281
        let mut t: halfword = 0; // §281
        if (self.cur_level > 1i32) {
            {
                'l_done_f: {
                    self.cur_level = (self.cur_level).wrapping_sub(1i32);
                    // §282
                    while true {
                        {
                            self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                            if (self.save_stack[(self.save_ptr) as usize].hh().b0() == 3i32) {
                                break 'l_done_f;
                            }
                            p = self.save_stack[(self.save_ptr) as usize].hh().rh();
                            if (self.save_stack[(self.save_ptr) as usize].hh().b0() == 2i32) {
                                // §326
                                {
                                    t = self.cur_tok;
                                    self.cur_tok = p;
                                    self.back_input();
                                    self.cur_tok = t;
                                }
                            } else {
                                // §282
                                {
                                    if (self.save_stack[(self.save_ptr) as usize].hh().b0() == 0i32) {
                                        {
                                            l = self.save_stack[(self.save_ptr) as usize].hh().b1();
                                            self.save_ptr = (self.save_ptr).wrapping_sub(1i32);
                                        }
                                    } else {
                                        { let __ix112 = self.save_ptr; let __v113 = self.eqtb[((2881i32) - 1) as usize]; self.save_stack[(__ix112) as usize] = __v113; }
                                    }
                                    // §283
                                    if (p < 5263i32) {
                                        if (self.eqtb[((p) - 1) as usize].hh().b1() == 1i32) {
                                            {
                                                self.eq_destroy(self.save_stack[(self.save_ptr) as usize]);
                                                if (self.eqtb[((5300i32) - 1) as usize].int() > 0i32) {
                                                    self.restore_trace(p, 544i32);
                                                }
                                            }
                                        } else {
                                            {
                                                self.eq_destroy(self.eqtb[((p) - 1) as usize]);
                                                { let __v114 = self.save_stack[(self.save_ptr) as usize]; self.eqtb[((p) - 1) as usize] = __v114; }
                                                if (self.eqtb[((5300i32) - 1) as usize].int() > 0i32) {
                                                    self.restore_trace(p, 545i32);
                                                }
                                            }
                                        }
                                    } else {
                                        if (self.xeq_level[((p) - 5263) as usize] != 1i32) {
                                            {
                                                { let __v115 = self.save_stack[(self.save_ptr) as usize]; self.eqtb[((p) - 1) as usize] = __v115; }
                                                self.xeq_level[((p) - 5263) as usize] = l;
                                                if (self.eqtb[((5300i32) - 1) as usize].int() > 0i32) {
                                                    self.restore_trace(p, 545i32);
                                                }
                                            }
                                        } else {
                                            {
                                                if (self.eqtb[((5300i32) - 1) as usize].int() > 0i32) {
                                                    self.restore_trace(p, 544i32);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // §282
                self.cur_group = self.save_stack[(self.save_ptr) as usize].hh().b1();
                self.cur_boundary = self.save_stack[(self.save_ptr) as usize].hh().rh();
            }
        } else {
            // §281
            self.confusion(543i32);
        }
    }

    /// The `prepare_mag` subroutine is called whenever \TeX\ wants to use `mag`
    /// for magnification.
    // §288
    pub fn prepare_mag(&mut self) {
        if ((self.mag_set > 0i32) && (self.eqtb[((5280i32) - 1) as usize].int() != self.mag_set)) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(547i32);
                }
                self.print_int(self.eqtb[((5280i32) - 1) as usize].int());
                self.print(548i32);
                self.print_nl(549i32);
                {
                    self.help_ptr = 2i32;
                    self.help_line[(1i32) as usize] = 550i32;
                    self.help_line[(0i32) as usize] = 551i32;
                }
                self.int_error(self.mag_set);
                self.geq_word_define(5280i32, self.mag_set);
            }
        }
        if ((self.eqtb[((5280i32) - 1) as usize].int() <= 0i32) || (self.eqtb[((5280i32) - 1) as usize].int() > 32768i32)) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(552i32);
                }
                {
                    self.help_ptr = 1i32;
                    self.help_line[(0i32) as usize] = 553i32;
                }
                self.int_error(self.eqtb[((5280i32) - 1) as usize].int());
                self.geq_word_define(5280i32, 1000i32);
            }
        }
        self.mag_set = self.eqtb[((5280i32) - 1) as usize].int();
    }

    /// Here's the way we sometimes want to display a token list, given a pointer
    /// to its reference count; the pointer may be null.
    // §295
    pub fn token_show(&mut self, mut p: halfword) {
        if (p != 0i32) {
            self.show_token_list(self.mem[(p) as usize].hh().rh(), 0i32, 10000000i32);
        }
    }

    /// The `print_meaning` subroutine displays `cur_cmd` and `cur_chr` in
    /// symbolic form, including the expansion of a macro or mark.
    // §296
    pub fn print_meaning(&mut self) {
        self.print_cmd_chr(self.cur_cmd, self.cur_chr);
        if (self.cur_cmd >= 111i32) {
            {
                self.print_char(58i32);
                self.print_ln();
                self.token_show(self.cur_chr);
            }
        } else {
            if (self.cur_cmd == 110i32) {
                {
                    self.print_char(58i32);
                    self.print_ln();
                    self.token_show(self.cur_mark[(self.cur_chr) as usize]);
                }
            }
        }
    }

    /// Here is a procedure that displays the current command.
    // §299
    pub fn show_cur_cmd_chr(&mut self) {
        self.begin_diagnostic();
        self.print_nl(123i32);
        if (self.cur_list.mode_field != self.shown_mode) {
            {
                self.print_mode(self.cur_list.mode_field);
                self.print(568i32);
                self.shown_mode = self.cur_list.mode_field;
            }
        }
        self.print_cmd_chr(self.cur_cmd, self.cur_chr);
        self.print_char(125i32);
        self.end_diagnostic(false);
    }

    /// The status at each level is indicated by printing two lines, where the first
    /// line indicates what was read so far and the second line shows what remains
    /// to be read. The context is cropped, if necessary, so that the first line
    /// contains at most `half_error_line` characters, and the second contains
    /// at most `error_line`. Non-current input levels whose `token_type` is
    /// ``backed_up`' are shown only if they have not been fully read.
    // §311
    pub fn show_context(&mut self) {
        let mut old_setting: i32 = 0; // §311
        let mut nn: i32 = 0; // §311
        let mut bottom_line: bool = false; // §311
        let mut i: i32 = 0; // §315
        let mut j: i32 = 0; // §315
        let mut l: i32 = 0; // §315
        let mut m: i32 = 0; // §315
        let mut n: i32 = 0; // §315
        let mut p: i32 = 0; // §315
        let mut q: i32 = 0; // §315
        'l_done_f: {
            self.base_ptr = self.input_ptr;
            { let __ix116 = self.base_ptr; let __v117 = self.cur_input; self.input_stack[(__ix116) as usize] = __v117; }
            nn = (1i32).wrapping_neg();
            bottom_line = false;
            while true {
                {
                    self.cur_input = self.input_stack[(self.base_ptr) as usize];
                    if (self.cur_input.state_field != 0i32) {
                        if ((self.cur_input.name_field > 17i32) || (self.base_ptr == 0i32)) {
                            bottom_line = true;
                        }
                    }
                    if (((self.base_ptr == self.input_ptr) || bottom_line) || (nn < self.eqtb[((5317i32) - 1) as usize].int())) {
                        // §312
                        {
                            if ((((self.base_ptr == self.input_ptr) || (self.cur_input.state_field != 0i32)) || (self.cur_input.index_field != 3i32)) || (self.cur_input.loc_field != 0i32)) {
                                {
                                    self.tally = 0i32;
                                    old_setting = self.selector;
                                    if (self.cur_input.state_field != 0i32) {
                                        {
                                            // §313
                                            if (self.cur_input.name_field <= 17i32) {
                                                if (self.cur_input.name_field == 0i32) {
                                                    if (self.base_ptr == 0i32) {
                                                        self.print_nl(574i32);
                                                    } else {
                                                        self.print_nl(575i32);
                                                    }
                                                } else {
                                                    {
                                                        self.print_nl(576i32);
                                                        if (self.cur_input.name_field == 17i32) {
                                                            self.print_char(42i32);
                                                        } else {
                                                            self.print_int((self.cur_input.name_field).wrapping_sub(1i32));
                                                        }
                                                        self.print_char(62i32);
                                                    }
                                                }
                                            } else {
                                                {
                                                    self.print_nl(577i32);
                                                    self.print_int(self.line);
                                                }
                                            }
                                            self.print_char(32i32);
                                            // §318
                                            {
                                                l = self.tally;
                                                self.tally = 0i32;
                                                self.selector = 20i32;
                                                self.trick_count = 1000000i32;
                                            }
                                            if (self.buffer[(self.cur_input.limit_field) as usize] == self.eqtb[((5311i32) - 1) as usize].int()) {
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
                                                                    self.trick_count = (((self.tally).wrapping_add(1i32)).wrapping_add(error_line)).wrapping_sub(half_error_line);
                                                                    if (self.trick_count < error_line) {
                                                                        self.trick_count = error_line;
                                                                    }
                                                                }
                                                            }
                                                            self.print(self.buffer[(i) as usize]);
                                                        }
                                                        i = i.wrapping_add(1);
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        // §312
                                        {
                                            // §314
                                            match self.cur_input.index_field {
                                                0 => {
                                                    self.print_nl(578i32);
                                                }
                                                1 | 2 => {
                                                    self.print_nl(579i32);
                                                }
                                                3 => {
                                                    if (self.cur_input.loc_field == 0i32) {
                                                        self.print_nl(580i32);
                                                    } else {
                                                        self.print_nl(581i32);
                                                    }
                                                }
                                                4 => {
                                                    self.print_nl(582i32);
                                                }
                                                5 => {
                                                    {
                                                        self.print_ln();
                                                        self.print_cs(self.cur_input.name_field);
                                                    }
                                                }
                                                6 => {
                                                    self.print_nl(583i32);
                                                }
                                                7 => {
                                                    self.print_nl(584i32);
                                                }
                                                8 => {
                                                    self.print_nl(585i32);
                                                }
                                                9 => {
                                                    self.print_nl(586i32);
                                                }
                                                10 => {
                                                    self.print_nl(587i32);
                                                }
                                                11 => {
                                                    self.print_nl(588i32);
                                                }
                                                12 => {
                                                    self.print_nl(589i32);
                                                }
                                                13 => {
                                                    self.print_nl(590i32);
                                                }
                                                14 => {
                                                    self.print_nl(591i32);
                                                }
                                                15 => {
                                                    self.print_nl(592i32);
                                                }
                                                _ => {
                                                    self.print_nl(63i32);
                                                }
                                            }
                                            // §319
                                            {
                                                l = self.tally;
                                                self.tally = 0i32;
                                                self.selector = 20i32;
                                                self.trick_count = 1000000i32;
                                            }
                                            if (self.cur_input.index_field < 5i32) {
                                                self.show_token_list(self.cur_input.start_field, self.cur_input.loc_field, 100000i32);
                                            } else {
                                                self.show_token_list(self.mem[(self.cur_input.start_field) as usize].hh().rh(), self.cur_input.loc_field, 100000i32);
                                            }
                                        }
                                    }
                                    // §312
                                    self.selector = old_setting;
                                    // §317
                                    if (self.trick_count == 1000000i32) {
                                        {
                                            self.first_count = self.tally;
                                            self.trick_count = (((self.tally).wrapping_add(1i32)).wrapping_add(error_line)).wrapping_sub(half_error_line);
                                            if (self.trick_count < error_line) {
                                                self.trick_count = error_line;
                                            }
                                        }
                                    }
                                    if (self.tally < self.trick_count) {
                                        m = (self.tally).wrapping_sub(self.first_count);
                                    } else {
                                        m = (self.trick_count).wrapping_sub(self.first_count);
                                    }
                                    if ((l).wrapping_add(self.first_count) <= half_error_line) {
                                        {
                                            p = 0i32;
                                            n = (l).wrapping_add(self.first_count);
                                        }
                                    } else {
                                        {
                                            self.print(277i32);
                                            p = (((l).wrapping_add(self.first_count)).wrapping_sub(half_error_line)).wrapping_add(3i32);
                                            n = half_error_line;
                                        }
                                    }
                                    {
                                        let __for_end_9 = (self.first_count).wrapping_sub(1i32);
                                        q = p;
                                        while q <= __for_end_9 {
                                            self.print_char(self.trick_buf[((q % error_line)) as usize]);
                                            q = q.wrapping_add(1);
                                        }
                                    }
                                    self.print_ln();
                                    {
                                        let __for_end_9 = n;
                                        q = 1i32;
                                        while q <= __for_end_9 {
                                            self.print_char(32i32);
                                            q = q.wrapping_add(1);
                                        }
                                    }
                                    if ((m).wrapping_add(n) <= error_line) {
                                        p = (self.first_count).wrapping_add(m);
                                    } else {
                                        p = (self.first_count).wrapping_add(((error_line).wrapping_sub(n)).wrapping_sub(3i32));
                                    }
                                    {
                                        let __for_end_9 = (p).wrapping_sub(1i32);
                                        q = self.first_count;
                                        while q <= __for_end_9 {
                                            self.print_char(self.trick_buf[((q % error_line)) as usize]);
                                            q = q.wrapping_add(1);
                                        }
                                    }
                                    if ((m).wrapping_add(n) > error_line) {
                                        self.print(277i32);
                                    }
                                    // §312
                                    nn = (nn).wrapping_add(1i32);
                                }
                            }
                        }
                    } else {
                        // §311
                        if (nn == self.eqtb[((5317i32) - 1) as usize].int()) {
                            {
                                self.print_nl(277i32);
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
        self.cur_input = self.input_stack[(self.input_ptr) as usize];
    }

    /// Here is a procedure that starts a new level of token-list input, given
    /// a token list `p` and its type `t`. If `t=macro`, the calling routine should
    /// set `name` and `loc`.
    // §323
    pub fn begin_token_list(&mut self, mut p: halfword, mut t: quarterword) {
        {
            if (self.input_ptr > self.max_in_stack) {
                {
                    self.max_in_stack = self.input_ptr;
                    if (self.input_ptr == stack_size) {
                        self.overflow(593i32, stack_size);
                    }
                }
            }
            { let __ix118 = self.input_ptr; let __v119 = self.cur_input; self.input_stack[(__ix118) as usize] = __v119; }
            self.input_ptr = (self.input_ptr).wrapping_add(1i32);
        }
        self.cur_input.state_field = 0i32;
        self.cur_input.start_field = p;
        self.cur_input.index_field = t;
        if (t >= 5i32) {
            {
                { let __v120 = (self.mem[(p) as usize].hh().lh()).wrapping_add(1i32); self.mem[(p) as usize].set_hh_lh(__v120); }
                if (t == 5i32) {
                    self.cur_input.limit_field = self.param_ptr;
                } else {
                    {
                        self.cur_input.loc_field = self.mem[(p) as usize].hh().rh();
                        if (self.eqtb[((5293i32) - 1) as usize].int() > 1i32) {
                            {
                                self.begin_diagnostic();
                                self.print_nl(338i32);
                                match t {
                                    14 => {
                                        self.print_esc(351i32);
                                    }
                                    15 => {
                                        self.print_esc(594i32);
                                    }
                                    _ => {
                                        self.print_cmd_chr(72i32, (t).wrapping_add(3407i32));
                                    }
                                }
                                self.print(556i32);
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
    // §324
    pub fn end_token_list(&mut self) {
        if (self.cur_input.index_field >= 3i32) {
            {
                if (self.cur_input.index_field <= 4i32) {
                    self.flush_list(self.cur_input.start_field);
                } else {
                    {
                        self.delete_token_ref(self.cur_input.start_field);
                        if (self.cur_input.index_field == 5i32) {
                            while (self.param_ptr > self.cur_input.limit_field) {
                                {
                                    self.param_ptr = (self.param_ptr).wrapping_sub(1i32);
                                    self.flush_list(self.param_stack[(self.param_ptr) as usize]);
                                }
                            }
                        }
                    }
                }
            }
        } else {
            if (self.cur_input.index_field == 1i32) {
                if (self.align_state > 500000i32) {
                    self.align_state = 0i32;
                } else {
                    self.fatal_error(595i32);
                }
            }
        }
        {
            self.input_ptr = (self.input_ptr).wrapping_sub(1i32);
            self.cur_input = self.input_stack[(self.input_ptr) as usize];
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
    // §325
    pub fn back_input(&mut self) {
        let mut p: halfword = 0; // §325
        while (((self.cur_input.state_field == 0i32) && (self.cur_input.loc_field == 0i32)) && (self.cur_input.index_field != 2i32)) {
            self.end_token_list();
        }
        p = self.get_avail();
        { let __v121 = self.cur_tok; self.mem[(p) as usize].set_hh_lh(__v121); }
        if (self.cur_tok < 768i32) {
            if (self.cur_tok < 512i32) {
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
                        self.overflow(593i32, stack_size);
                    }
                }
            }
            { let __ix122 = self.input_ptr; let __v123 = self.cur_input; self.input_stack[(__ix122) as usize] = __v123; }
            self.input_ptr = (self.input_ptr).wrapping_add(1i32);
        }
        self.cur_input.state_field = 0i32;
        self.cur_input.start_field = p;
        self.cur_input.index_field = 3i32;
        self.cur_input.loc_field = p;
    }

    /// The `back_error` routine is used when we want to replace an offending token
    /// just before issuing an error message. This routine, like `back_input`,
    /// requires that `cur_tok` has been set. We disable interrupts during the
    /// call of `back_input` so that the help message won't be lost.
    // §327
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
    // §327
    pub fn ins_error(&mut self) {
        self.OK_to_interrupt = false;
        self.back_input();
        self.cur_input.index_field = 4i32;
        self.OK_to_interrupt = true;
        self.error();
    }

    /// The `begin_file_reading` procedure starts a new level of input for lines
    /// of characters to be read from a file, or as an insertion from the
    /// terminal. It does not take care of opening the file, nor does it set `loc`
    /// or `limit` or `line`.
    // §328
    pub fn begin_file_reading(&mut self) {
        if (self.in_open == max_in_open) {
            self.overflow(596i32, max_in_open);
        }
        if (self.first == buf_size) {
            self.overflow(256i32, buf_size);
        }
        self.in_open = (self.in_open).wrapping_add(1i32);
        {
            if (self.input_ptr > self.max_in_stack) {
                {
                    self.max_in_stack = self.input_ptr;
                    if (self.input_ptr == stack_size) {
                        self.overflow(593i32, stack_size);
                    }
                }
            }
            { let __ix124 = self.input_ptr; let __v125 = self.cur_input; self.input_stack[(__ix124) as usize] = __v125; }
            self.input_ptr = (self.input_ptr).wrapping_add(1i32);
        }
        self.cur_input.index_field = self.in_open;
        { let __ix126 = self.cur_input.index_field; let __v127 = self.line; self.line_stack[((__ix126) - 1) as usize] = __v127; }
        self.cur_input.start_field = self.first;
        self.cur_input.state_field = 1i32;
        self.cur_input.name_field = 0i32;
    }

    /// Conversely, the variables must be downdated when such a level of input
    /// is finished:
    // §329
    pub fn end_file_reading(&mut self) {
        self.first = self.cur_input.start_field;
        self.line = self.line_stack[((self.cur_input.index_field) - 1) as usize];
        if (self.cur_input.name_field > 17i32) {
            { let mut __f = ::core::mem::take(&mut self.input_file[((self.cur_input.index_field) - 1) as usize]); let __r = self.a_close(&mut __f); self.input_file[((self.cur_input.index_field) - 1) as usize] = __f; __r };
        }
        {
            self.input_ptr = (self.input_ptr).wrapping_sub(1i32);
            self.cur_input = self.input_stack[(self.input_ptr) as usize];
        }
        self.in_open = (self.in_open).wrapping_sub(1i32);
    }

    /// In order to keep the stack from overflowing during a long sequence of
    /// inserted `\.{\\show}' commands, the following routine removes completed
    /// error-inserted lines from memory.
    // §330
    pub fn clear_for_error_prompt(&mut self) {
        while ((((self.cur_input.state_field != 0i32) && (self.cur_input.name_field == 0i32)) && (self.input_ptr > 0i32)) && (self.cur_input.loc_field > self.cur_input.limit_field)) {
            self.end_file_reading();
        }
        self.print_ln();
        crate::system::break_in(&mut self.term_in, true);
    }

    /// Before getting into `get_next`, let's consider the subroutine that
    /// is called when an `\.{\\outer}' control sequence has been scanned or
    /// when the end of a file has been reached. These two cases are distinguished
    /// by `cur_cs`, which is zero at the end of a file.
    // §336
    pub fn check_outer_validity(&mut self) {
        let mut p: halfword = 0; // §336
        let mut q: halfword = 0; // §336
        if (self.scanner_status != 0i32) {
            {
                self.deletions_allowed = false;
                // §337
                if (self.cur_cs != 0i32) {
                    {
                        if (((self.cur_input.state_field == 0i32) || (self.cur_input.name_field < 1i32)) || (self.cur_input.name_field > 17i32)) {
                            {
                                p = self.get_avail();
                                { let __v128 = (4095i32).wrapping_add(self.cur_cs); self.mem[(p) as usize].set_hh_lh(__v128); }
                                self.begin_token_list(p, 3i32);
                            }
                        }
                        self.cur_cmd = 10i32;
                        self.cur_chr = 32i32;
                    }
                }
                // §336
                if (self.scanner_status > 1i32) {
                    // §338
                    {
                        self.runaway();
                        if (self.cur_cs == 0i32) {
                            {
                                if (self.interaction == 3i32) {
                                }
                                self.print_nl(262i32);
                                self.print(604i32);
                            }
                        } else {
                            {
                                self.cur_cs = 0i32;
                                {
                                    if (self.interaction == 3i32) {
                                    }
                                    self.print_nl(262i32);
                                    self.print(605i32);
                                }
                            }
                        }
                        self.print(606i32);
                        // §339
                        p = self.get_avail();
                        match self.scanner_status {
                            2 => {
                                {
                                    self.print(570i32);
                                    self.mem[(p) as usize].set_hh_lh(637i32);
                                }
                            }
                            3 => {
                                {
                                    self.print(612i32);
                                    { let __v129 = self.par_token; self.mem[(p) as usize].set_hh_lh(__v129); }
                                    self.long_state = 113i32;
                                }
                            }
                            4 => {
                                {
                                    self.print(572i32);
                                    self.mem[(p) as usize].set_hh_lh(637i32);
                                    q = p;
                                    p = self.get_avail();
                                    self.mem[(p) as usize].set_hh_rh(q);
                                    self.mem[(p) as usize].set_hh_lh(6710i32);
                                    self.align_state = (1000000i32).wrapping_neg();
                                }
                            }
                            5 => {
                                {
                                    self.print(573i32);
                                    self.mem[(p) as usize].set_hh_lh(637i32);
                                }
                            }
                            _ => {}
                        }
                        self.begin_token_list(p, 4i32);
                        // §338
                        self.print(607i32);
                        self.sprint_cs(self.warning_index);
                        {
                            self.help_ptr = 4i32;
                            self.help_line[(3i32) as usize] = 608i32;
                            self.help_line[(2i32) as usize] = 609i32;
                            self.help_line[(1i32) as usize] = 610i32;
                            self.help_line[(0i32) as usize] = 611i32;
                        }
                        self.error();
                    }
                } else {
                    // §336
                    {
                        {
                            if (self.interaction == 3i32) {
                            }
                            self.print_nl(262i32);
                            self.print(598i32);
                        }
                        self.print_cmd_chr(105i32, self.cur_if);
                        self.print(599i32);
                        self.print_int(self.skip_line);
                        {
                            self.help_ptr = 3i32;
                            self.help_line[(2i32) as usize] = 600i32;
                            self.help_line[(1i32) as usize] = 601i32;
                            self.help_line[(0i32) as usize] = 602i32;
                        }
                        if (self.cur_cs != 0i32) {
                            self.cur_cs = 0i32;
                        } else {
                            self.help_line[(2i32) as usize] = 603i32;
                        }
                        self.cur_tok = 6713i32;
                        self.ins_error();
                    }
                }
                self.deletions_allowed = true;
            }
        }
    }

    /// Now we're ready to take the plunge into `get_next` itself. Parts of
    /// this routine are executed more often than any other instructions of \TeX.
    // §341
    pub fn get_next(&mut self) {
        let mut k: i32 = 0; // §341
        let mut t: halfword = 0; // §341
        let mut cat: i32 = 0; // §341
        let mut c: ASCII_code = 0; // §341
        let mut cc: ASCII_code = 0; // §341
        let mut d: i32 = 0; // §341
        // goto labels: restart, exit
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                self.cur_cs = 0i32;
                if (self.cur_input.state_field != 0i32) {
                    // §343
                    {
                        'l_L25_b: loop {
                            if (self.cur_input.loc_field <= self.cur_input.limit_field) {
                                {
                                    self.cur_chr = self.buffer[(self.cur_input.loc_field) as usize];
                                    self.cur_input.loc_field = (self.cur_input.loc_field).wrapping_add(1i32);
                                    'l_reswitch_b: loop {
                                        self.cur_cmd = self.eqtb[(((3983i32).wrapping_add(self.cur_chr)) - 1) as usize].hh().rh();
                                        // §344
                                        match (self.cur_input.state_field).wrapping_add(self.cur_cmd) {
                                            10 | 26 | 42 | 27 | 43 => {
                                                continue 'l_L25_b;
                                            }
                                            1 | 17 | 33 => {
                                                // §354
                                                {
                                                    'l_found_f: {
                                                        if (self.cur_input.loc_field > self.cur_input.limit_field) {
                                                            self.cur_cs = 513i32;
                                                        } else {
                                                            {
                                                                'l_L26_b: loop {
                                                                    k = self.cur_input.loc_field;
                                                                    self.cur_chr = self.buffer[(k) as usize];
                                                                    cat = self.eqtb[(((3983i32).wrapping_add(self.cur_chr)) - 1) as usize].hh().rh();
                                                                    k = (k).wrapping_add(1i32);
                                                                    if (cat == 11i32) {
                                                                        self.cur_input.state_field = 17i32;
                                                                    } else {
                                                                        if (cat == 10i32) {
                                                                            self.cur_input.state_field = 17i32;
                                                                        } else {
                                                                            self.cur_input.state_field = 1i32;
                                                                        }
                                                                    }
                                                                    if ((cat == 11i32) && (k <= self.cur_input.limit_field)) {
                                                                        // §356
                                                                        {
                                                                            loop {
                                                                                self.cur_chr = self.buffer[(k) as usize];
                                                                                cat = self.eqtb[(((3983i32).wrapping_add(self.cur_chr)) - 1) as usize].hh().rh();
                                                                                k = (k).wrapping_add(1i32);
                                                                                if ((cat != 11i32) || (k > self.cur_input.limit_field)) { break; }
                                                                            }
                                                                            // §355
                                                                            {
                                                                                if (self.buffer[(k) as usize] == self.cur_chr) {
                                                                                    if (cat == 7i32) {
                                                                                        if (k < self.cur_input.limit_field) {
                                                                                            {
                                                                                                c = self.buffer[((k).wrapping_add(1i32)) as usize];
                                                                                                if (c < 128i32) {
                                                                                                    {
                                                                                                        d = 2i32;
                                                                                                        if (((c >= 48i32) && (c <= 57i32)) || ((c >= 97i32) && (c <= 102i32))) {
                                                                                                            if ((k).wrapping_add(2i32) <= self.cur_input.limit_field) {
                                                                                                                {
                                                                                                                    cc = self.buffer[((k).wrapping_add(2i32)) as usize];
                                                                                                                    if (((cc >= 48i32) && (cc <= 57i32)) || ((cc >= 97i32) && (cc <= 102i32))) {
                                                                                                                        d = (d).wrapping_add(1i32);
                                                                                                                    }
                                                                                                                }
                                                                                                            }
                                                                                                        }
                                                                                                        if (d > 2i32) {
                                                                                                            {
                                                                                                                if (c <= 57i32) {
                                                                                                                    self.cur_chr = (c).wrapping_sub(48i32);
                                                                                                                } else {
                                                                                                                    self.cur_chr = (c).wrapping_sub(87i32);
                                                                                                                }
                                                                                                                if (cc <= 57i32) {
                                                                                                                    self.cur_chr = (((16i32).wrapping_mul(self.cur_chr)).wrapping_add(cc)).wrapping_sub(48i32);
                                                                                                                } else {
                                                                                                                    self.cur_chr = (((16i32).wrapping_mul(self.cur_chr)).wrapping_add(cc)).wrapping_sub(87i32);
                                                                                                                }
                                                                                                                { let __v130 = self.cur_chr; self.buffer[((k).wrapping_sub(1i32)) as usize] = __v130; }
                                                                                                            }
                                                                                                        } else {
                                                                                                            if (c < 64i32) {
                                                                                                                self.buffer[((k).wrapping_sub(1i32)) as usize] = (c).wrapping_add(64i32);
                                                                                                            } else {
                                                                                                                self.buffer[((k).wrapping_sub(1i32)) as usize] = (c).wrapping_sub(64i32);
                                                                                                            }
                                                                                                        }
                                                                                                        self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(d);
                                                                                                        self.first = (self.first).wrapping_sub(d);
                                                                                                        while (k <= self.cur_input.limit_field) {
                                                                                                            {
                                                                                                                { let __v131 = self.buffer[((k).wrapping_add(d)) as usize]; self.buffer[(k) as usize] = __v131; }
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
                                                                            // §356
                                                                            if (cat != 11i32) {
                                                                                k = (k).wrapping_sub(1i32);
                                                                            }
                                                                            if (k > (self.cur_input.loc_field).wrapping_add(1i32)) {
                                                                                {
                                                                                    self.cur_cs = self.id_lookup(self.cur_input.loc_field, (k).wrapping_sub(self.cur_input.loc_field));
                                                                                    self.cur_input.loc_field = k;
                                                                                    break 'l_found_f;
                                                                                }
                                                                            }
                                                                        }
                                                                    } else {
                                                                        // §355
                                                                        {
                                                                            if (self.buffer[(k) as usize] == self.cur_chr) {
                                                                                if (cat == 7i32) {
                                                                                    if (k < self.cur_input.limit_field) {
                                                                                        {
                                                                                            c = self.buffer[((k).wrapping_add(1i32)) as usize];
                                                                                            if (c < 128i32) {
                                                                                                {
                                                                                                    d = 2i32;
                                                                                                    if (((c >= 48i32) && (c <= 57i32)) || ((c >= 97i32) && (c <= 102i32))) {
                                                                                                        if ((k).wrapping_add(2i32) <= self.cur_input.limit_field) {
                                                                                                            {
                                                                                                                cc = self.buffer[((k).wrapping_add(2i32)) as usize];
                                                                                                                if (((cc >= 48i32) && (cc <= 57i32)) || ((cc >= 97i32) && (cc <= 102i32))) {
                                                                                                                    d = (d).wrapping_add(1i32);
                                                                                                                }
                                                                                                            }
                                                                                                        }
                                                                                                    }
                                                                                                    if (d > 2i32) {
                                                                                                        {
                                                                                                            if (c <= 57i32) {
                                                                                                                self.cur_chr = (c).wrapping_sub(48i32);
                                                                                                            } else {
                                                                                                                self.cur_chr = (c).wrapping_sub(87i32);
                                                                                                            }
                                                                                                            if (cc <= 57i32) {
                                                                                                                self.cur_chr = (((16i32).wrapping_mul(self.cur_chr)).wrapping_add(cc)).wrapping_sub(48i32);
                                                                                                            } else {
                                                                                                                self.cur_chr = (((16i32).wrapping_mul(self.cur_chr)).wrapping_add(cc)).wrapping_sub(87i32);
                                                                                                            }
                                                                                                            { let __v132 = self.cur_chr; self.buffer[((k).wrapping_sub(1i32)) as usize] = __v132; }
                                                                                                        }
                                                                                                    } else {
                                                                                                        if (c < 64i32) {
                                                                                                            self.buffer[((k).wrapping_sub(1i32)) as usize] = (c).wrapping_add(64i32);
                                                                                                        } else {
                                                                                                            self.buffer[((k).wrapping_sub(1i32)) as usize] = (c).wrapping_sub(64i32);
                                                                                                        }
                                                                                                    }
                                                                                                    self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(d);
                                                                                                    self.first = (self.first).wrapping_sub(d);
                                                                                                    while (k <= self.cur_input.limit_field) {
                                                                                                        {
                                                                                                            { let __v133 = self.buffer[((k).wrapping_add(d)) as usize]; self.buffer[(k) as usize] = __v133; }
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
                                                                    // §354
                                                                    self.cur_cs = (257i32).wrapping_add(self.buffer[(self.cur_input.loc_field) as usize]);
                                                                    self.cur_input.loc_field = (self.cur_input.loc_field).wrapping_add(1i32);
                                                                    break 'l_L26_b;
                                                                }
                                                            }
                                                        }
                                                    }
                                                    self.cur_cmd = self.eqtb[((self.cur_cs) - 1) as usize].hh().b0();
                                                    self.cur_chr = self.eqtb[((self.cur_cs) - 1) as usize].hh().rh();
                                                    if (self.cur_cmd >= 113i32) {
                                                        self.check_outer_validity();
                                                    }
                                                }
                                            }
                                            14 | 30 | 46 => {
                                                // §353
                                                {
                                                    self.cur_cs = (self.cur_chr).wrapping_add(1i32);
                                                    self.cur_cmd = self.eqtb[((self.cur_cs) - 1) as usize].hh().b0();
                                                    self.cur_chr = self.eqtb[((self.cur_cs) - 1) as usize].hh().rh();
                                                    self.cur_input.state_field = 1i32;
                                                    if (self.cur_cmd >= 113i32) {
                                                        self.check_outer_validity();
                                                    }
                                                }
                                            }
                                            8 | 24 | 40 => {
                                                // §352
                                                {
                                                    if (self.cur_chr == self.buffer[(self.cur_input.loc_field) as usize]) {
                                                        if (self.cur_input.loc_field < self.cur_input.limit_field) {
                                                            {
                                                                c = self.buffer[((self.cur_input.loc_field).wrapping_add(1i32)) as usize];
                                                                if (c < 128i32) {
                                                                    {
                                                                        self.cur_input.loc_field = (self.cur_input.loc_field).wrapping_add(2i32);
                                                                        if (((c >= 48i32) && (c <= 57i32)) || ((c >= 97i32) && (c <= 102i32))) {
                                                                            if (self.cur_input.loc_field <= self.cur_input.limit_field) {
                                                                                {
                                                                                    cc = self.buffer[(self.cur_input.loc_field) as usize];
                                                                                    if (((cc >= 48i32) && (cc <= 57i32)) || ((cc >= 97i32) && (cc <= 102i32))) {
                                                                                        {
                                                                                            self.cur_input.loc_field = (self.cur_input.loc_field).wrapping_add(1i32);
                                                                                            if (c <= 57i32) {
                                                                                                self.cur_chr = (c).wrapping_sub(48i32);
                                                                                            } else {
                                                                                                self.cur_chr = (c).wrapping_sub(87i32);
                                                                                            }
                                                                                            if (cc <= 57i32) {
                                                                                                self.cur_chr = (((16i32).wrapping_mul(self.cur_chr)).wrapping_add(cc)).wrapping_sub(48i32);
                                                                                            } else {
                                                                                                self.cur_chr = (((16i32).wrapping_mul(self.cur_chr)).wrapping_add(cc)).wrapping_sub(87i32);
                                                                                            }
                                                                                            continue 'l_reswitch_b;
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                        if (c < 64i32) {
                                                                            self.cur_chr = (c).wrapping_add(64i32);
                                                                        } else {
                                                                            self.cur_chr = (c).wrapping_sub(64i32);
                                                                        }
                                                                        continue 'l_reswitch_b;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    self.cur_input.state_field = 1i32;
                                                }
                                            }
                                            16 | 32 | 48 => {
                                                // §346
                                                {
                                                    {
                                                        if (self.interaction == 3i32) {
                                                        }
                                                        self.print_nl(262i32);
                                                        self.print(613i32);
                                                    }
                                                    {
                                                        self.help_ptr = 2i32;
                                                        self.help_line[(1i32) as usize] = 614i32;
                                                        self.help_line[(0i32) as usize] = 615i32;
                                                    }
                                                    self.deletions_allowed = false;
                                                    self.error();
                                                    self.deletions_allowed = true;
                                                    { __goto_1 = 0; continue 'l_dispatch_1; }
                                                }
                                            }
                                            11 => {
                                                // §349
                                                {
                                                    self.cur_input.state_field = 17i32;
                                                    self.cur_chr = 32i32;
                                                }
                                            }
                                            6 => {
                                                // §348
                                                {
                                                    self.cur_input.loc_field = (self.cur_input.limit_field).wrapping_add(1i32);
                                                    self.cur_cmd = 10i32;
                                                    self.cur_chr = 32i32;
                                                }
                                            }
                                            22 | 15 | 31 | 47 => {
                                                // §350
                                                {
                                                    self.cur_input.loc_field = (self.cur_input.limit_field).wrapping_add(1i32);
                                                    continue 'l_L25_b;
                                                }
                                            }
                                            38 => {
                                                // §351
                                                {
                                                    self.cur_input.loc_field = (self.cur_input.limit_field).wrapping_add(1i32);
                                                    self.cur_cs = self.par_loc;
                                                    self.cur_cmd = self.eqtb[((self.cur_cs) - 1) as usize].hh().b0();
                                                    self.cur_chr = self.eqtb[((self.cur_cs) - 1) as usize].hh().rh();
                                                    if (self.cur_cmd >= 113i32) {
                                                        self.check_outer_validity();
                                                    }
                                                }
                                            }
                                            2 => {
                                                // §347
                                                self.align_state = (self.align_state).wrapping_add(1i32);
                                            }
                                            18 | 34 => {
                                                {
                                                    self.cur_input.state_field = 1i32;
                                                    self.align_state = (self.align_state).wrapping_add(1i32);
                                                }
                                            }
                                            3 => {
                                                self.align_state = (self.align_state).wrapping_sub(1i32);
                                            }
                                            19 | 35 => {
                                                {
                                                    self.cur_input.state_field = 1i32;
                                                    self.align_state = (self.align_state).wrapping_sub(1i32);
                                                }
                                            }
                                            20 | 21 | 23 | 25 | 28 | 29 | 36 | 37 | 39 | 41 | 44 | 45 => {
                                                self.cur_input.state_field = 1i32;
                                            }
                                            _ => {
                                                // §344
                                            }
                                        }
                                        break 'l_reswitch_b;
                                    }
                                }
                            } else {
                                // §343
                                {
                                    self.cur_input.state_field = 33i32;
                                    // §360
                                    if (self.cur_input.name_field > 17i32) {
                                        // §362
                                        {
                                            self.line = (self.line).wrapping_add(1i32);
                                            self.first = self.cur_input.start_field;
                                            if (!self.force_eof) {
                                                {
                                                    if { let mut __f = ::core::mem::take(&mut self.input_file[((self.cur_input.index_field) - 1) as usize]); let __r = self.input_ln(&mut __f, true); self.input_file[((self.cur_input.index_field) - 1) as usize] = __f; __r } {
                                                        self.firm_up_the_line();
                                                    } else {
                                                        self.force_eof = true;
                                                    }
                                                }
                                            }
                                            if self.force_eof {
                                                {
                                                    self.print_char(41i32);
                                                    self.open_parens = (self.open_parens).wrapping_sub(1i32);
                                                    crate::system::break_out(&mut self.term_out);
                                                    self.force_eof = false;
                                                    self.end_file_reading();
                                                    self.check_outer_validity();
                                                    { __goto_1 = 0; continue 'l_dispatch_1; }
                                                }
                                            }
                                            if ((self.eqtb[((5311i32) - 1) as usize].int() < 0i32) || (self.eqtb[((5311i32) - 1) as usize].int() > 255i32)) {
                                                self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(1i32);
                                            } else {
                                                { let __ix134 = self.cur_input.limit_field; let __v135 = self.eqtb[((5311i32) - 1) as usize].int(); self.buffer[(__ix134) as usize] = __v135; }
                                            }
                                            self.first = (self.cur_input.limit_field).wrapping_add(1i32);
                                            self.cur_input.loc_field = self.cur_input.start_field;
                                        }
                                    } else {
                                        // §360
                                        {
                                            if (!(self.cur_input.name_field == 0i32)) {
                                                {
                                                    self.cur_cmd = 0i32;
                                                    self.cur_chr = 0i32;
                                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                                }
                                            }
                                            if (self.input_ptr > 0i32) {
                                                {
                                                    self.end_file_reading();
                                                    { __goto_1 = 0; continue 'l_dispatch_1; }
                                                }
                                            }
                                            if (self.selector < 18i32) {
                                                self.open_log_file();
                                            }
                                            if (self.interaction > 1i32) {
                                                {
                                                    if ((self.eqtb[((5311i32) - 1) as usize].int() < 0i32) || (self.eqtb[((5311i32) - 1) as usize].int() > 255i32)) {
                                                        self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_add(1i32);
                                                    }
                                                    if (self.cur_input.limit_field == self.cur_input.start_field) {
                                                        self.print_nl(616i32);
                                                    }
                                                    self.print_ln();
                                                    self.first = self.cur_input.start_field;
                                                    {
                                                        self.print(42i32);
                                                        self.term_input();
                                                    }
                                                    self.cur_input.limit_field = self.last;
                                                    if ((self.eqtb[((5311i32) - 1) as usize].int() < 0i32) || (self.eqtb[((5311i32) - 1) as usize].int() > 255i32)) {
                                                        self.cur_input.limit_field = (self.cur_input.limit_field).wrapping_sub(1i32);
                                                    } else {
                                                        { let __ix136 = self.cur_input.limit_field; let __v137 = self.eqtb[((5311i32) - 1) as usize].int(); self.buffer[(__ix136) as usize] = __v137; }
                                                    }
                                                    self.first = (self.cur_input.limit_field).wrapping_add(1i32);
                                                    self.cur_input.loc_field = self.cur_input.start_field;
                                                }
                                            } else {
                                                self.fatal_error(617i32);
                                            }
                                        }
                                    }
                                    // §343
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
                    // §357
                    if (self.cur_input.loc_field != 0i32) {
                        {
                            t = self.mem[(self.cur_input.loc_field) as usize].hh().lh();
                            self.cur_input.loc_field = self.mem[(self.cur_input.loc_field) as usize].hh().rh();
                            if (t >= 4095i32) {
                                {
                                    self.cur_cs = (t).wrapping_sub(4095i32);
                                    self.cur_cmd = self.eqtb[((self.cur_cs) - 1) as usize].hh().b0();
                                    self.cur_chr = self.eqtb[((self.cur_cs) - 1) as usize].hh().rh();
                                    if (self.cur_cmd >= 113i32) {
                                        if (self.cur_cmd == 116i32) {
                                            // §358
                                            {
                                                self.cur_cs = (self.mem[(self.cur_input.loc_field) as usize].hh().lh()).wrapping_sub(4095i32);
                                                self.cur_input.loc_field = 0i32;
                                                self.cur_cmd = self.eqtb[((self.cur_cs) - 1) as usize].hh().b0();
                                                self.cur_chr = self.eqtb[((self.cur_cs) - 1) as usize].hh().rh();
                                                if (self.cur_cmd > 100i32) {
                                                    {
                                                        self.cur_cmd = 0i32;
                                                        self.cur_chr = 257i32;
                                                    }
                                                }
                                            }
                                        } else {
                                            // §357
                                            self.check_outer_validity();
                                        }
                                    }
                                }
                            } else {
                                {
                                    self.cur_cmd = (t / 256i32);
                                    self.cur_chr = (t % 256i32);
                                    match self.cur_cmd {
                                        1 => {
                                            self.align_state = (self.align_state).wrapping_add(1i32);
                                        }
                                        2 => {
                                            self.align_state = (self.align_state).wrapping_sub(1i32);
                                        }
                                        5 => {
                                            // §359
                                            {
                                                self.begin_token_list(self.param_stack[(((self.cur_input.limit_field).wrapping_add(self.cur_chr)).wrapping_sub(1i32)) as usize], 0i32);
                                                { __goto_1 = 0; continue 'l_dispatch_1; }
                                            }
                                        }
                                        _ => {
                                            // §357
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        {
                            self.end_token_list();
                            { __goto_1 = 0; continue 'l_dispatch_1; }
                        }
                    }
                }
                // §342
                if (self.cur_cmd <= 5i32) {
                    if (self.cur_cmd >= 4i32) {
                        if (self.align_state == 0i32) {
                            // §789
                            {
                                if ((self.scanner_status == 4i32) || (self.cur_align == 0i32)) {
                                    self.fatal_error(595i32);
                                }
                                self.cur_cmd = self.mem[((self.cur_align).wrapping_add(5i32)) as usize].hh().lh();
                                { let __ix138 = (self.cur_align).wrapping_add(5i32); let __v139 = self.cur_chr; self.mem[(__ix138) as usize].set_hh_lh(__v139); }
                                if (self.cur_cmd == 63i32) {
                                    self.begin_token_list(29990i32, 2i32);
                                } else {
                                    self.begin_token_list(self.mem[((self.cur_align).wrapping_add(2i32)) as usize].int(), 2i32);
                                }
                                self.align_state = 1000000i32;
                                { __goto_1 = 0; continue 'l_dispatch_1; }
                            }
                        }
                    }
                }
            }
            if __goto_1 <= 1 { // exit
                // §341
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
    // §363
    pub fn firm_up_the_line(&mut self) {
        let mut k: i32 = 0; // §363
        self.cur_input.limit_field = self.last;
        if (self.eqtb[((5291i32) - 1) as usize].int() > 0i32) {
            if (self.interaction > 1i32) {
                {
                    self.print_ln();
                    if (self.cur_input.start_field < self.cur_input.limit_field) {
                        {
                            let __for_end_6 = (self.cur_input.limit_field).wrapping_sub(1i32);
                            k = self.cur_input.start_field;
                            while k <= __for_end_6 {
                                self.print(self.buffer[(k) as usize]);
                                k = k.wrapping_add(1);
                            }
                        }
                    }
                    self.first = self.cur_input.limit_field;
                    {
                        self.print(618i32);
                        self.term_input();
                    }
                    if (self.last > self.first) {
                        {
                            {
                                let __for_end_7 = (self.last).wrapping_sub(1i32);
                                k = self.first;
                                while k <= __for_end_7 {
                                    { let __ix140 = ((k).wrapping_add(self.cur_input.start_field)).wrapping_sub(self.first); let __v141 = self.buffer[(k) as usize]; self.buffer[(__ix140) as usize] = __v141; }
                                    k = k.wrapping_add(1);
                                }
                            }
                            self.cur_input.limit_field = ((self.cur_input.start_field).wrapping_add(self.last)).wrapping_sub(self.first);
                        }
                    }
                }
            }
        }
    }

    /// No new control sequences will be defined except during a call of
    /// `get_token`, or when \.{\\csname} compresses a token list, because
    /// `no_new_control_sequence` is always `true` at other times.
    // §365
    pub fn get_token(&mut self) {
        self.no_new_control_sequence = false;
        self.get_next();
        self.no_new_control_sequence = true;
        if (self.cur_cs == 0i32) {
            self.cur_tok = ((self.cur_cmd).wrapping_mul(256i32)).wrapping_add(self.cur_chr);
        } else {
            self.cur_tok = (4095i32).wrapping_add(self.cur_cs);
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
    // §389
    pub fn macro_call(&mut self) {
        let mut r: halfword = 0; // §389
        let mut p: halfword = 0; // §389
        let mut q: halfword = 0; // §389
        let mut s: halfword = 0; // §389
        let mut t: halfword = 0; // §389
        let mut u: halfword = 0; // §389
        let mut v: halfword = 0; // §389
        let mut rbrace_ptr: halfword = 0; // §389
        let mut n: small_number = 0; // §389
        let mut unbalance: halfword = 0; // §389
        let mut m: halfword = 0; // §389
        let mut ref_count: halfword = 0; // §389
        let mut save_scanner_status: small_number = 0; // §389
        let mut save_warning_index: halfword = 0; // §389
        let mut match_chr: ASCII_code = 0; // §389
        'l_exit_f: {
            save_scanner_status = self.scanner_status;
            save_warning_index = self.warning_index;
            self.warning_index = self.cur_cs;
            ref_count = self.cur_chr;
            r = self.mem[(ref_count) as usize].hh().rh();
            n = 0i32;
            if (self.eqtb[((5293i32) - 1) as usize].int() > 0i32) {
                // §401
                {
                    self.begin_diagnostic();
                    self.print_ln();
                    self.print_cs(self.warning_index);
                    self.token_show(ref_count);
                    self.end_diagnostic(false);
                }
            }
            // §389
            if (self.mem[(r) as usize].hh().lh() != 3584i32) {
                // §391
                {
                    self.scanner_status = 3i32;
                    unbalance = 0i32;
                    self.long_state = self.eqtb[((self.cur_cs) - 1) as usize].hh().b0();
                    if (self.long_state >= 113i32) {
                        self.long_state = (self.long_state).wrapping_sub(2i32);
                    }
                    loop {
                        // goto labels: continue, found
                        let mut __goto_1: i32 = 0;
                        'l_dispatch_1: loop {
                            if __goto_1 <= 0 {
                                self.mem[(29997i32) as usize].set_hh_rh(0i32);
                                if ((self.mem[(r) as usize].hh().lh() > 3583i32) || (self.mem[(r) as usize].hh().lh() < 3328i32)) {
                                    s = 0i32;
                                } else {
                                    {
                                        match_chr = (self.mem[(r) as usize].hh().lh()).wrapping_sub(3328i32);
                                        s = self.mem[(r) as usize].hh().rh();
                                        r = s;
                                        p = 29997i32;
                                        m = 0i32;
                                    }
                                }
                            }
                            if __goto_1 <= 1 { // continue
                                // §392
                                self.get_token();
                                if (self.cur_tok == self.mem[(r) as usize].hh().lh()) {
                                    // §394
                                    {
                                        r = self.mem[(r) as usize].hh().rh();
                                        if ((self.mem[(r) as usize].hh().lh() >= 3328i32) && (self.mem[(r) as usize].hh().lh() <= 3584i32)) {
                                            {
                                                if (self.cur_tok < 512i32) {
                                                    self.align_state = (self.align_state).wrapping_sub(1i32);
                                                }
                                                { __goto_1 = 2; continue 'l_dispatch_1; }
                                            }
                                        } else {
                                            { __goto_1 = 1; continue 'l_dispatch_1; }
                                        }
                                    }
                                }
                                // §397
                                if (s != r) {
                                    if (s == 0i32) {
                                        // §398
                                        {
                                            {
                                                if (self.interaction == 3i32) {
                                                }
                                                self.print_nl(262i32);
                                                self.print(650i32);
                                            }
                                            self.sprint_cs(self.warning_index);
                                            self.print(651i32);
                                            {
                                                self.help_ptr = 4i32;
                                                self.help_line[(3i32) as usize] = 652i32;
                                                self.help_line[(2i32) as usize] = 653i32;
                                                self.help_line[(1i32) as usize] = 654i32;
                                                self.help_line[(0i32) as usize] = 655i32;
                                            }
                                            self.error();
                                            break 'l_exit_f;
                                        }
                                    } else {
                                        // §397
                                        {
                                            t = s;
                                            loop {
                                                'l_done_f: {
                                                    {
                                                        q = self.get_avail();
                                                        self.mem[(p) as usize].set_hh_rh(q);
                                                        { let __v142 = self.mem[(t) as usize].hh().lh(); self.mem[(q) as usize].set_hh_lh(__v142); }
                                                        p = q;
                                                    }
                                                    m = (m).wrapping_add(1i32);
                                                    u = self.mem[(t) as usize].hh().rh();
                                                    v = s;
                                                    while true {
                                                        {
                                                            if (u == r) {
                                                                if (self.cur_tok != self.mem[(v) as usize].hh().lh()) {
                                                                    break 'l_done_f;
                                                                } else {
                                                                    {
                                                                        r = self.mem[(v) as usize].hh().rh();
                                                                        { __goto_1 = 1; continue 'l_dispatch_1; }
                                                                    }
                                                                }
                                                            }
                                                            if (self.mem[(u) as usize].hh().lh() != self.mem[(v) as usize].hh().lh()) {
                                                                break 'l_done_f;
                                                            }
                                                            u = self.mem[(u) as usize].hh().rh();
                                                            v = self.mem[(v) as usize].hh().rh();
                                                        }
                                                    }
                                                }
                                                t = self.mem[(t) as usize].hh().rh();
                                                if (t == r) { break; }
                                            }
                                            r = s;
                                        }
                                    }
                                }
                                // §392
                                if (self.cur_tok == self.par_token) {
                                    if (self.long_state != 112i32) {
                                        // §396
                                        {
                                            if (self.long_state == 111i32) {
                                                {
                                                    self.runaway();
                                                    {
                                                        if (self.interaction == 3i32) {
                                                        }
                                                        self.print_nl(262i32);
                                                        self.print(645i32);
                                                    }
                                                    self.sprint_cs(self.warning_index);
                                                    self.print(646i32);
                                                    {
                                                        self.help_ptr = 3i32;
                                                        self.help_line[(2i32) as usize] = 647i32;
                                                        self.help_line[(1i32) as usize] = 648i32;
                                                        self.help_line[(0i32) as usize] = 649i32;
                                                    }
                                                    self.back_error();
                                                }
                                            }
                                            { let __v143 = self.mem[(29997i32) as usize].hh().rh(); self.pstack[(n) as usize] = __v143; }
                                            self.align_state = (self.align_state).wrapping_sub(unbalance);
                                            {
                                                let __for_end_11 = n;
                                                m = 0i32;
                                                while m <= __for_end_11 {
                                                    self.flush_list(self.pstack[(m) as usize]);
                                                    m = m.wrapping_add(1);
                                                }
                                            }
                                            break 'l_exit_f;
                                        }
                                    }
                                }
                                // §392
                                if (self.cur_tok < 768i32) {
                                    if (self.cur_tok < 512i32) {
                                        // §399
                                        {
                                            'l_done1_f: {
                                                unbalance = 1i32;
                                                while true {
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
                                                            { let __v144 = self.cur_tok; self.mem[(q) as usize].set_hh_lh(__v144); }
                                                            p = q;
                                                        }
                                                        self.get_token();
                                                        if (self.cur_tok == self.par_token) {
                                                            if (self.long_state != 112i32) {
                                                                // §396
                                                                {
                                                                    if (self.long_state == 111i32) {
                                                                        {
                                                                            self.runaway();
                                                                            {
                                                                                if (self.interaction == 3i32) {
                                                                                }
                                                                                self.print_nl(262i32);
                                                                                self.print(645i32);
                                                                            }
                                                                            self.sprint_cs(self.warning_index);
                                                                            self.print(646i32);
                                                                            {
                                                                                self.help_ptr = 3i32;
                                                                                self.help_line[(2i32) as usize] = 647i32;
                                                                                self.help_line[(1i32) as usize] = 648i32;
                                                                                self.help_line[(0i32) as usize] = 649i32;
                                                                            }
                                                                            self.back_error();
                                                                        }
                                                                    }
                                                                    { let __v145 = self.mem[(29997i32) as usize].hh().rh(); self.pstack[(n) as usize] = __v145; }
                                                                    self.align_state = (self.align_state).wrapping_sub(unbalance);
                                                                    {
                                                                        let __for_end_17 = n;
                                                                        m = 0i32;
                                                                        while m <= __for_end_17 {
                                                                            self.flush_list(self.pstack[(m) as usize]);
                                                                            m = m.wrapping_add(1);
                                                                        }
                                                                    }
                                                                    break 'l_exit_f;
                                                                }
                                                            }
                                                        }
                                                        // §399
                                                        if (self.cur_tok < 768i32) {
                                                            if (self.cur_tok < 512i32) {
                                                                unbalance = (unbalance).wrapping_add(1i32);
                                                            } else {
                                                                {
                                                                    unbalance = (unbalance).wrapping_sub(1i32);
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
                                                self.mem[(p) as usize].set_hh_rh(q);
                                                { let __v146 = self.cur_tok; self.mem[(q) as usize].set_hh_lh(__v146); }
                                                p = q;
                                            }
                                        }
                                    } else {
                                        // §395
                                        {
                                            self.back_input();
                                            {
                                                if (self.interaction == 3i32) {
                                                }
                                                self.print_nl(262i32);
                                                self.print(637i32);
                                            }
                                            self.sprint_cs(self.warning_index);
                                            self.print(638i32);
                                            {
                                                self.help_ptr = 6i32;
                                                self.help_line[(5i32) as usize] = 639i32;
                                                self.help_line[(4i32) as usize] = 640i32;
                                                self.help_line[(3i32) as usize] = 641i32;
                                                self.help_line[(2i32) as usize] = 642i32;
                                                self.help_line[(1i32) as usize] = 643i32;
                                                self.help_line[(0i32) as usize] = 644i32;
                                            }
                                            self.align_state = (self.align_state).wrapping_add(1i32);
                                            self.long_state = 111i32;
                                            self.cur_tok = self.par_token;
                                            self.ins_error();
                                            { __goto_1 = 1; continue 'l_dispatch_1; }
                                        }
                                    }
                                } else {
                                    // §393
                                    {
                                        if (self.cur_tok == 2592i32) {
                                            if (self.mem[(r) as usize].hh().lh() <= 3584i32) {
                                                if (self.mem[(r) as usize].hh().lh() >= 3328i32) {
                                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                                }
                                            }
                                        }
                                        {
                                            q = self.get_avail();
                                            self.mem[(p) as usize].set_hh_rh(q);
                                            { let __v147 = self.cur_tok; self.mem[(q) as usize].set_hh_lh(__v147); }
                                            p = q;
                                        }
                                    }
                                }
                                // §392
                                m = (m).wrapping_add(1i32);
                                if (self.mem[(r) as usize].hh().lh() > 3584i32) {
                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                }
                                if (self.mem[(r) as usize].hh().lh() < 3328i32) {
                                    { __goto_1 = 1; continue 'l_dispatch_1; }
                                }
                            }
                            if __goto_1 <= 2 { // found
                                if (s != 0i32) {
                                    // §400
                                    {
                                        if ((m == 1i32) && (self.mem[(p) as usize].hh().lh() < 768i32)) {
                                            {
                                                self.mem[(rbrace_ptr) as usize].set_hh_rh(0i32);
                                                {
                                                    { let __v148 = self.avail; self.mem[(p) as usize].set_hh_rh(__v148); }
                                                    self.avail = p;
                                                    self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                }
                                                p = self.mem[(29997i32) as usize].hh().rh();
                                                { let __v149 = self.mem[(p) as usize].hh().rh(); self.pstack[(n) as usize] = __v149; }
                                                {
                                                    { let __v150 = self.avail; self.mem[(p) as usize].set_hh_rh(__v150); }
                                                    self.avail = p;
                                                    self.dyn_used = (self.dyn_used).wrapping_sub(1i32);
                                                }
                                            }
                                        } else {
                                            { let __v151 = self.mem[(29997i32) as usize].hh().rh(); self.pstack[(n) as usize] = __v151; }
                                        }
                                        n = (n).wrapping_add(1i32);
                                        if (self.eqtb[((5293i32) - 1) as usize].int() > 0i32) {
                                            {
                                                self.begin_diagnostic();
                                                self.print_nl(match_chr);
                                                self.print_int(n);
                                                self.print(656i32);
                                                self.show_token_list(self.pstack[((n).wrapping_sub(1i32)) as usize], 0i32, 1000i32);
                                                self.end_diagnostic(false);
                                            }
                                        }
                                    }
                                }
                            }
                            break 'l_dispatch_1;
                        }
                        if (self.mem[(r) as usize].hh().lh() == 3584i32) { break; }
                    }
                }
            }
            // §390
            while (((self.cur_input.state_field == 0i32) && (self.cur_input.loc_field == 0i32)) && (self.cur_input.index_field != 2i32)) {
                self.end_token_list();
            }
            self.begin_token_list(ref_count, 5i32);
            self.cur_input.name_field = self.warning_index;
            self.cur_input.loc_field = self.mem[(r) as usize].hh().rh();
            if (n > 0i32) {
                {
                    if ((self.param_ptr).wrapping_add(n) > self.max_param_stack) {
                        {
                            self.max_param_stack = (self.param_ptr).wrapping_add(n);
                            if (self.max_param_stack > param_size) {
                                self.overflow(636i32, param_size);
                            }
                        }
                    }
                    {
                        let __for_end_5 = (n).wrapping_sub(1i32);
                        m = 0i32;
                        while m <= __for_end_5 {
                            { let __ix152 = (self.param_ptr).wrapping_add(m); let __v153 = self.pstack[(m) as usize]; self.param_stack[(__ix152) as usize] = __v153; }
                            m = m.wrapping_add(1);
                        }
                    }
                    self.param_ptr = (self.param_ptr).wrapping_add(n);
                }
            }
        }
        // §389
        self.scanner_status = save_scanner_status;
        self.warning_index = save_warning_index;
    }

    /// Sometimes the expansion looks too far ahead, so we want to insert
    /// a harmless \.{\\relax} into the user's input.
    /// @<Declare the procedure called `insert_relax`
    // §379
    pub fn insert_relax(&mut self) {
        self.cur_tok = (4095i32).wrapping_add(self.cur_cs);
        self.back_input();
        self.cur_tok = 6716i32;
        self.back_input();
        self.cur_input.index_field = 4i32;
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
    // §366
    pub fn expand(&mut self) {
        let mut t: halfword = 0; // §366
        let mut p: halfword = 0; // §366
        let mut q: halfword = 0; // §366
        let mut r: halfword = 0; // §366
        let mut j: i32 = 0; // §366
        let mut cv_backup: i32 = 0; // §366
        let mut cvl_backup: small_number = 0; // §366
        let mut radix_backup: small_number = 0; // §366
        let mut co_backup: small_number = 0; // §366
        let mut backup_backup: halfword = 0; // §366
        let mut save_scanner_status: small_number = 0; // §366
        cv_backup = self.cur_val;
        cvl_backup = self.cur_val_level;
        radix_backup = self.radix;
        co_backup = self.cur_order;
        backup_backup = self.mem[(29987i32) as usize].hh().rh();
        if (self.cur_cmd < 111i32) {
            // §367
            {
                if (self.eqtb[((5299i32) - 1) as usize].int() > 1i32) {
                    self.show_cur_cmd_chr();
                }
                match self.cur_cmd {
                    110 => {
                        // §386
                        {
                            if (self.cur_mark[(self.cur_chr) as usize] != 0i32) {
                                self.begin_token_list(self.cur_mark[(self.cur_chr) as usize], 14i32);
                            }
                        }
                    }
                    102 => {
                        // §368
                        {
                            self.get_token();
                            t = self.cur_tok;
                            self.get_token();
                            if (self.cur_cmd > 100i32) {
                                self.expand();
                            } else {
                                self.back_input();
                            }
                            self.cur_tok = t;
                            self.back_input();
                        }
                    }
                    103 => {
                        // §369
                        {
                            save_scanner_status = self.scanner_status;
                            self.scanner_status = 0i32;
                            self.get_token();
                            self.scanner_status = save_scanner_status;
                            t = self.cur_tok;
                            self.back_input();
                            if (t >= 4095i32) {
                                {
                                    p = self.get_avail();
                                    self.mem[(p) as usize].set_hh_lh(6718i32);
                                    { let __v154 = self.cur_input.loc_field; self.mem[(p) as usize].set_hh_rh(__v154); }
                                    self.cur_input.start_field = p;
                                    self.cur_input.loc_field = p;
                                }
                            }
                        }
                    }
                    107 => {
                        // §372
                        {
                            r = self.get_avail();
                            p = r;
                            loop {
                                self.get_x_token();
                                if (self.cur_cs == 0i32) {
                                    {
                                        q = self.get_avail();
                                        self.mem[(p) as usize].set_hh_rh(q);
                                        { let __v155 = self.cur_tok; self.mem[(q) as usize].set_hh_lh(__v155); }
                                        p = q;
                                    }
                                }
                                if (self.cur_cs != 0i32) { break; }
                            }
                            if (self.cur_cmd != 67i32) {
                                // §373
                                {
                                    {
                                        if (self.interaction == 3i32) {
                                        }
                                        self.print_nl(262i32);
                                        self.print(625i32);
                                    }
                                    self.print_esc(505i32);
                                    self.print(626i32);
                                    {
                                        self.help_ptr = 2i32;
                                        self.help_line[(1i32) as usize] = 627i32;
                                        self.help_line[(0i32) as usize] = 628i32;
                                    }
                                    self.back_error();
                                }
                            }
                            // §374
                            j = self.first;
                            p = self.mem[(r) as usize].hh().rh();
                            while (p != 0i32) {
                                {
                                    if (j >= self.max_buf_stack) {
                                        {
                                            self.max_buf_stack = (j).wrapping_add(1i32);
                                            if (self.max_buf_stack == buf_size) {
                                                self.overflow(256i32, buf_size);
                                            }
                                        }
                                    }
                                    { let __v156 = (self.mem[(p) as usize].hh().lh() % 256i32); self.buffer[(j) as usize] = __v156; }
                                    j = (j).wrapping_add(1i32);
                                    p = self.mem[(p) as usize].hh().rh();
                                }
                            }
                            if (j > (self.first).wrapping_add(1i32)) {
                                {
                                    self.no_new_control_sequence = false;
                                    self.cur_cs = self.id_lookup(self.first, (j).wrapping_sub(self.first));
                                    self.no_new_control_sequence = true;
                                }
                            } else {
                                if (j == self.first) {
                                    self.cur_cs = 513i32;
                                } else {
                                    self.cur_cs = (257i32).wrapping_add(self.buffer[(self.first) as usize]);
                                }
                            }
                            // §372
                            self.flush_list(r);
                            if (self.eqtb[((self.cur_cs) - 1) as usize].hh().b0() == 101i32) {
                                {
                                    self.eq_define(self.cur_cs, 0i32, 256i32);
                                }
                            }
                            self.cur_tok = (self.cur_cs).wrapping_add(4095i32);
                            self.back_input();
                        }
                    }
                    108 => {
                        // §367
                        self.conv_toks();
                    }
                    109 => {
                        self.ins_the_toks();
                    }
                    105 => {
                        self.conditional();
                    }
                    106 => {
                        // §510
                        if (self.cur_chr > self.if_limit) {
                            if (self.if_limit == 1i32) {
                                self.insert_relax();
                            } else {
                                {
                                    {
                                        if (self.interaction == 3i32) {
                                        }
                                        self.print_nl(262i32);
                                        self.print(777i32);
                                    }
                                    self.print_cmd_chr(106i32, self.cur_chr);
                                    {
                                        self.help_ptr = 1i32;
                                        self.help_line[(0i32) as usize] = 778i32;
                                    }
                                    self.error();
                                }
                            }
                        } else {
                            {
                                while (self.cur_chr != 2i32) {
                                    self.pass_text();
                                }
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
                    104 => {
                        // §378
                        if (self.cur_chr > 0i32) {
                            self.force_eof = true;
                        } else {
                            if self.name_in_progress {
                                self.insert_relax();
                            } else {
                                self.start_input();
                            }
                        }
                    }
                    _ => {
                        // §370
                        {
                            {
                                if (self.interaction == 3i32) {
                                }
                                self.print_nl(262i32);
                                self.print(619i32);
                            }
                            {
                                self.help_ptr = 5i32;
                                self.help_line[(4i32) as usize] = 620i32;
                                self.help_line[(3i32) as usize] = 621i32;
                                self.help_line[(2i32) as usize] = 622i32;
                                self.help_line[(1i32) as usize] = 623i32;
                                self.help_line[(0i32) as usize] = 624i32;
                            }
                            self.error();
                        }
                    }
                }
            }
        } else {
            // §366
            if (self.cur_cmd < 115i32) {
                self.macro_call();
            } else {
                // §375
                {
                    self.cur_tok = 6715i32;
                    self.back_input();
                }
            }
        }
        // §366
        self.cur_val = cv_backup;
        self.cur_val_level = cvl_backup;
        self.radix = radix_backup;
        self.cur_order = co_backup;
        self.mem[(29987i32) as usize].set_hh_rh(backup_backup);
    }

    /// Here is a recursive procedure that is \TeX's usual way to get the
    /// next token of input. It has been slightly optimized to take account of
    /// common cases.
    // §380
    pub fn get_x_token(&mut self) {
        // goto labels: restart, done
        let mut __goto_1: i32 = 0;
        'l_dispatch_1: loop {
            if __goto_1 <= 0 {
                self.get_next();
                if (self.cur_cmd <= 100i32) {
                    { __goto_1 = 1; continue 'l_dispatch_1; }
                }
                if (self.cur_cmd >= 111i32) {
                    if (self.cur_cmd < 115i32) {
                        self.macro_call();
                    } else {
                        {
                            self.cur_cs = 2620i32;
                            self.cur_cmd = 9i32;
                            { __goto_1 = 1; continue 'l_dispatch_1; }
                        }
                    }
                } else {
                    self.expand();
                }
                { __goto_1 = 0; continue 'l_dispatch_1; }
            }
            if __goto_1 <= 1 { // done
                if (self.cur_cs == 0i32) {
                    self.cur_tok = ((self.cur_cmd).wrapping_mul(256i32)).wrapping_add(self.cur_chr);
                } else {
                    self.cur_tok = (4095i32).wrapping_add(self.cur_cs);
                }
            }
            break 'l_dispatch_1;
        }
    }

    /// The `get_x_token` procedure is essentially equivalent to two consecutive
    /// procedure calls: `get_next; x_token`.
    // §381
    pub fn x_token(&mut self) {
        while (self.cur_cmd > 100i32) {
            {
                self.expand();
                self.get_next();
            }
        }
        if (self.cur_cs == 0i32) {
            self.cur_tok = ((self.cur_cmd).wrapping_mul(256i32)).wrapping_add(self.cur_chr);
        } else {
            self.cur_tok = (4095i32).wrapping_add(self.cur_cs);
        }
    }

    /// The `scan_left_brace` routine is called when a left brace is supposed to be
    /// the next non-blank token. (The term ``left brace'' means, more precisely,
    /// a character whose catcode is `left_brace`.) \TeX\ allows \.{\\relax} to
    /// appear before the `left_brace`.
    // §403
    pub fn scan_left_brace(&mut self) {
        // §404
        loop {
            self.get_x_token();
            if ((self.cur_cmd != 10i32) && (self.cur_cmd != 0i32)) { break; }
        }
        // §403
        if (self.cur_cmd != 1i32) {
            {
                {
                    if (self.interaction == 3i32) {
                    }
                    self.print_nl(262i32);
                    self.print(657i32);
                }
                {
                    self.help_ptr = 4i32;
                    self.help_line[(3i32) as usize] = 658i32;
                    self.help_line[(2i32) as usize] = 659i32;
                    self.help_line[(1i32) as usize] = 660i32;
                    self.help_line[(0i32) as usize] = 661i32;
                }
                self.back_error();
                self.cur_tok = 379i32;
                self.cur_cmd = 1i32;
                self.cur_chr = 123i32;
                self.align_state = (self.align_state).wrapping_add(1i32);
            }
        }
    }

}
