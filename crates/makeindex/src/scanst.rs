//! scanst.c and scanst.h: reading the style file.

use crate::io::{mk_getc, EOF};
use crate::locale::tolower_c;
use crate::mkind::*;

const COMMENT: i32 = b'%' as i32;
const STR_DELIM: i32 = b'"' as i32;
const CHR_DELIM: i32 = b'\'' as i32;
const BSH: i32 = b'\\' as i32;

const ROMAN_LOWER_OFFSET: i32 = 10000;
const ROMAN_UPPER_OFFSET: i32 = 10000;
const ARABIC_OFFSET: i32 = 10000;
const ALPHA_LOWER_OFFSET: i32 = 26;
const ALPHA_UPPER_OFFSET: i32 = 26;

/// Which string attribute `scan_string` sets.
#[derive(Clone, Copy)]
enum S {
    Preamble,
    Postamble,
    GroupSkip,
    HeadingPre,
    HeadingSuf,
    SymheadPos,
    SymheadNeg,
    NumheadPos,
    NumheadNeg,
    SetpageOpen,
    SetpageClose,
    ItemR(usize),
    ItemU(usize),
    ItemX(usize),
    EncapPrefix,
    EncapInfix,
    EncapSuffix,
    DelimP(usize),
    DelimN,
    DelimR,
    DelimT,
    Suffix2p,
    Suffix3p,
    SuffixMp,
    IndentSpace,
    PageComp,
    PagePrec,
    Keyword,
}

/// Which character attribute `scan_char` sets.
#[derive(Clone, Copy)]
enum C {
    Aopen,
    Aclose,
    Level,
    Ropen,
    Rclose,
    Quote,
    Actual,
    Encap,
    Escape,
}

fn count_lfd(s: &[u8]) -> i32 {
    s.iter().filter(|&&b| b == b'\n').count() as i32
}

impl Mk<'_> {
    fn sty_getc(&mut self) -> i32 {
        mk_getc(&mut self.lookahead, self.sty_fp.as_mut().unwrap())
    }

    /// `STY_ERROR` and friends.
    fn sty_error(&mut self, msg: &[u8]) {
        if self.idx_dot {
            self.ilg(b"\n");
            self.idx_dot = false;
        }
        let h = fmt(
            "** Input style error (file = %s, line = %d):\n   -- ",
            &[P::S(&self.sty_fn.clone()), self.sty_lc.into()],
        );
        self.ilg(&h);
        self.ilg(msg);
        self.sty_ec += 1;
        self.put_dot = false;
    }

    fn sty_dot(&mut self) {
        self.idx_dot = true;
        if self.verbose {
            self.err(b".");
        }
        self.ilg(b".");
    }

    fn sty_skipline(&mut self) {
        loop {
            let a = self.sty_getc();
            if a == LFD || a == EOF {
                break;
            }
        }
        self.sty_lc += 1;
    }

    /// `SCAN_NO(N)`: `fscanf(sty_fp, "%d", N)`, around `mk_getc`.
    fn scan_number(&mut self, dst: &mut i32) {
        self.sty_fp.as_mut().unwrap().scan_int(dst);
    }

    fn str_mut(&mut self, s: S) -> &mut Vec<u8> {
        match s {
            S::Preamble => &mut self.preamble,
            S::Postamble => &mut self.postamble,
            S::GroupSkip => &mut self.group_skip,
            S::HeadingPre => &mut self.heading_pre,
            S::HeadingSuf => &mut self.heading_suf,
            S::SymheadPos => &mut self.symhead_pos,
            S::SymheadNeg => &mut self.symhead_neg,
            S::NumheadPos => &mut self.numhead_pos,
            S::NumheadNeg => &mut self.numhead_neg,
            S::SetpageOpen => &mut self.setpage_open,
            S::SetpageClose => &mut self.setpage_close,
            S::ItemR(k) => &mut self.item_r[k],
            S::ItemU(k) => &mut self.item_u[k],
            S::ItemX(k) => &mut self.item_x[k],
            S::EncapPrefix => &mut self.encap_p,
            S::EncapInfix => &mut self.encap_i,
            S::EncapSuffix => &mut self.encap_s,
            S::DelimP(k) => &mut self.delim_p[k],
            S::DelimN => &mut self.delim_n,
            S::DelimR => &mut self.delim_r,
            S::DelimT => &mut self.delim_t,
            S::Suffix2p => &mut self.suffix_2p,
            S::Suffix3p => &mut self.suffix_3p,
            S::SuffixMp => &mut self.suffix_mp,
            S::IndentSpace => &mut self.indent_space,
            S::PageComp => &mut self.page_comp,
            S::PagePrec => &mut self.page_prec,
            S::Keyword => &mut self.idx_keyword,
        }
    }

    fn chr_mut(&mut self, c: C) -> &mut u8 {
        match c {
            C::Aopen => &mut self.idx_aopen,
            C::Aclose => &mut self.idx_aclose,
            C::Level => &mut self.idx_level,
            C::Ropen => &mut self.idx_ropen,
            C::Rclose => &mut self.idx_rclose,
            C::Quote => &mut self.idx_quote,
            C::Actual => &mut self.idx_actual,
            C::Encap => &mut self.idx_encap,
            C::Escape => &mut self.idx_escape,
        }
    }

    /// `scan_string` then `count_lfd` of the result into `len`.
    fn scan_string_lfd(&mut self, s: S) -> i32 {
        self.scan_string(s);
        count_lfd(self.str_mut(s))
    }

    pub fn scan_sty(&mut self) {
        let m = fmt("Scanning style file %s", &[P::S(&self.sty_fn.clone())]);
        self.message(&m);
        loop {
            let (r, spec) = self.scan_spec();
            if r == 0 {
                break;
            }
            self.sty_tc += 1;
            self.put_dot = true;

            let sp = cstr(&spec).to_vec();
            match &sp[..] {
                // output pre- and post-ambles
                b"preamble" => self.prelen = self.scan_string_lfd(S::Preamble),
                b"postamble" => self.postlen = self.scan_string_lfd(S::Postamble),
                b"group_skip" => self.skiplen = self.scan_string_lfd(S::GroupSkip),
                b"headings_flag" => {
                    let mut h = self.headings_flag;
                    self.scan_number(&mut h);
                    self.headings_flag = h;
                }
                b"heading_prefix" => self.headprelen = self.scan_string_lfd(S::HeadingPre),
                b"heading_suffix" => self.headsuflen = self.scan_string_lfd(S::HeadingSuf),
                b"symhead_positive" => self.scan_string(S::SymheadPos),
                b"symhead_negative" => self.scan_string(S::SymheadNeg),
                b"numhead_positive" => self.scan_string(S::NumheadPos),
                b"numhead_negative" => self.scan_string(S::NumheadNeg),
                b"setpage_prefix" => self.setpagelen = self.scan_string_lfd(S::SetpageOpen),
                b"setpage_suffix" => self.setpagelen = self.scan_string_lfd(S::SetpageClose),
                // output index item commands
                b"item_0" => self.ilen_r[0] = self.scan_string_lfd(S::ItemR(0)),
                b"item_1" => self.ilen_r[1] = self.scan_string_lfd(S::ItemR(1)),
                b"item_2" => self.ilen_r[2] = self.scan_string_lfd(S::ItemR(2)),
                b"item_01" => self.ilen_u[1] = self.scan_string_lfd(S::ItemU(1)),
                b"item_12" => self.ilen_u[2] = self.scan_string_lfd(S::ItemU(2)),
                b"item_x1" => self.ilen_x[1] = self.scan_string_lfd(S::ItemX(1)),
                b"item_x2" => self.ilen_x[2] = self.scan_string_lfd(S::ItemX(2)),
                // output encapsulators
                b"encap_prefix" => self.scan_string(S::EncapPrefix),
                b"encap_infix" => self.scan_string(S::EncapInfix),
                b"encap_suffix" => self.scan_string(S::EncapSuffix),
                // output delimiters
                b"delim_0" => self.scan_string(S::DelimP(0)),
                b"delim_1" => self.scan_string(S::DelimP(1)),
                b"delim_2" => self.scan_string(S::DelimP(2)),
                b"delim_n" => self.scan_string(S::DelimN),
                b"delim_r" => self.scan_string(S::DelimR),
                b"delim_t" => self.scan_string(S::DelimT),
                b"suffix_2p" => self.scan_string(S::Suffix2p),
                b"suffix_3p" => self.scan_string(S::Suffix3p),
                b"suffix_mp" => self.scan_string(S::SuffixMp),
                // output line width
                b"line_max" => {
                    let mut tmp = self.sty_tmp;
                    self.scan_number(&mut tmp);
                    self.sty_tmp = tmp;
                    if tmp > 0 {
                        self.linemax = tmp;
                    } else {
                        let m = fmt(
                            "%s must be positive (got %d)",
                            &["line_max".into(), tmp.into()],
                        );
                        self.sty_error(&m);
                    }
                }
                // output line indentation length
                b"indent_length" => {
                    let mut tmp = self.sty_tmp;
                    self.scan_number(&mut tmp);
                    self.sty_tmp = tmp;
                    if tmp >= 0 {
                        self.indent_length = tmp;
                    } else {
                        let m = fmt(
                            "%s must be nonnegative (got %d)",
                            &["indent_length".into(), tmp.into()],
                        );
                        self.sty_error(&m);
                    }
                }
                // output line indentation
                b"indent_space" => self.scan_string(S::IndentSpace),
                // composite page delimiter
                b"page_compositor" => self.scan_string(S::PageComp),
                // page precedence
                b"page_precedence" => self.scan_string(S::PagePrec),
                // index input format
                b"keyword" => self.scan_string(S::Keyword),
                b"arg_open" => self.scan_char(C::Aopen),
                b"arg_close" => self.scan_char(C::Aclose),
                b"level" => self.scan_char(C::Level),
                b"range_open" => self.scan_char(C::Ropen),
                b"range_close" => self.scan_char(C::Rclose),
                b"quote" => self.scan_char(C::Quote),
                b"actual" => self.scan_char(C::Actual),
                b"encap" => self.scan_char(C::Encap),
                b"escape" => self.scan_char(C::Escape),
                _ => {
                    self.next_nonblank();
                    self.sty_skipline();
                    let m = fmt("Unknown specifier %s.\n", &[P::S(&sp)]);
                    self.sty_error(&m);
                    self.put_dot = false;
                }
            }
            if self.put_dot {
                self.sty_dot();
            }
        }
        // page precedence
        self.process_precedence();

        // check if quote and escape are distinct
        if self.idx_quote == self.idx_escape {
            let m = fmt(
                "Quote and escape symbols must be distinct (both `%c' now).\n",
                &[P::C(self.idx_quote)],
            );
            self.sty_error(&m);
            self.idx_quote = b'"';
            self.idx_escape = b'\\';
        }
        self.done(
            self.sty_tc - self.sty_ec,
            "attributes redefined",
            self.sty_ec,
            "ignored",
        );
        self.sty_fp = None;
    }

    /// `scan_spec`: its result (0, 1, or -1, which the caller takes as
    /// true) and the specifier.
    fn scan_spec(&mut self) -> (i32, Vec<u8>) {
        let mut spec = vec![0u8; STRING_MAX + 1];
        let mut c;
        loop {
            c = self.next_nonblank();
            if c == -1 {
                return (0, spec);
            } else if c == COMMENT {
                self.sty_skipline();
            } else {
                break;
            }
        }

        spec[0] = tolower_c(c as u8);
        let mut i = 0usize;
        loop {
            let ok = i < STRING_MAX;
            i += 1;
            if !ok {
                break;
            }
            c = self.sty_getc();
            if c == SPC || c == TAB || c == LFD || c == EOF {
                break;
            }
            spec[i] = tolower_c(c as u8);
        }
        if i < STRING_MAX {
            spec[i] = 0;
            if c == EOF {
                let m = fmt(
                    "No attribute for specifier %s (premature EOF)\n",
                    &[P::S(cstr(&spec))],
                );
                self.sty_error(&m);
                return (-1, spec);
            }
            if c == LFD {
                self.sty_lc += 1;
            }
            (1, spec)
        } else {
            let m = fmt(
                "Specifier %s too long (max %d).\n",
                &[P::S(cstr(&spec[..STRING_MAX])), STRING_MAX.into()],
            );
            self.sty_error(&m);
            (0, spec)
        }
    }

    fn next_nonblank(&mut self) -> i32 {
        loop {
            let c = self.sty_getc();
            if c == EOF {
                return -1;
            } else if c == LFD {
                self.sty_lc += 1;
            } else if c == SPC || c == TAB {
            } else {
                return c;
            }
        }
    }

    /// `clone[i++] = c` on the persistent stack buffer.
    fn clone_put(&mut self, i: &mut usize, c: u8) {
        if *i >= self.sty_clone.len() {
            self.sty_clone.resize(*i + 1, 0);
        }
        self.sty_clone[*i] = c;
        *i += 1;
    }

    fn scan_string(&mut self, which: S) {
        let mut i = 0usize;
        let c = self.next_nonblank();
        if c == STR_DELIM {
            loop {
                let c = self.sty_getc();
                if c == EOF {
                    // `clone` is not terminated here: the C program prints
                    // what earlier calls left after the bytes read.
                    let m = fmt(
                        "No closing delimiter in %s.\n",
                        &[P::S(cstr(&self.sty_clone.clone()))],
                    );
                    self.sty_error(&m);
                    return;
                } else if c == STR_DELIM {
                    self.clone_put(&mut i, 0);
                    let s = cstr(&self.sty_clone).to_vec();
                    if let S::Keyword = which {
                        // strcpy into char idx_keyword[ARRAY_MAX]
                        if s.len() >= self.idx_keyword.len() {
                            self.idx_keyword.resize(s.len() + 1, 0);
                        }
                        self.idx_keyword[..s.len()].copy_from_slice(&s);
                        self.idx_keyword[s.len()] = 0;
                    } else {
                        *self.str_mut(which) = s;
                    }
                    return;
                } else if c == BSH {
                    let c = self.sty_getc();
                    match c {
                        x if x == b't' as i32 => self.clone_put(&mut i, b'\t'),
                        x if x == b'n' as i32 => self.clone_put(&mut i, b'\n'),
                        _ => self.clone_put(&mut i, c as u8),
                    }
                } else {
                    if c == LFD {
                        self.sty_lc += 1;
                    }
                    if i < ARRAY_MAX {
                        self.clone_put(&mut i, c as u8);
                    } else {
                        self.sty_skipline();
                        let m = fmt(
                            "Attribute string %s too long (max %d).\n",
                            &[P::S(cstr(&self.sty_clone.clone())), ARRAY_MAX.into()],
                        );
                        self.sty_error(&m);
                        return;
                    }
                }
            }
        } else if c == COMMENT {
            self.sty_skipline();
        } else {
            self.sty_skipline();
            self.sty_error(b"No opening delimiter.\n");
        }
    }

    fn scan_char(&mut self, which: C) {
        let c = self.next_nonblank();
        if c == CHR_DELIM {
            let mut clone = self.sty_getc();
            if clone == CHR_DELIM {
                self.sty_skipline();
                self.sty_error(b"Premature closing delimiter.\n");
                return;
            }
            if clone == LFD || clone == EOF {
                if clone == LFD {
                    self.sty_lc += 1;
                }
                self.sty_error(b"No character (premature EOF).\n");
                return;
            }
            if clone == BSH {
                clone = self.sty_getc();
            }
            if self.sty_getc() == CHR_DELIM {
                *self.chr_mut(which) = clone as u8;
            } else {
                self.sty_error(b"No closing delimiter or too many letters.\n");
            }
        } else if c == COMMENT {
            self.sty_skipline();
        } else {
            self.sty_skipline();
            self.sty_error(b"No opening delimiter.\n");
        }
    }

    fn multiple(&mut self, c: u8) {
        self.sty_skipline();
        let m = fmt(
            "Multiple instances of type `%c' in page precedence specification `%s'.\n",
            &[P::C(c), P::S(&self.page_prec.clone())],
        );
        self.sty_error(&m);
    }

    fn process_precedence(&mut self) {
        let mut order = [0i32; PAGETYPE_MAX + 1];
        let mut type_ = [0i32; PAGETYPE_MAX + 1];
        let mut i = 0usize;
        let (mut roml, mut romu, mut arab, mut alpl, mut alpu) =
            (false, false, false, false, false);
        let pp = cstr(&self.page_prec).to_vec();

        // check for illegal specifiers first
        while i < PAGETYPE_MAX && at(&pp, i) != 0 {
            match at(&pp, i) {
                b'r' => {
                    if roml {
                        self.multiple(b'r');
                        return;
                    }
                    roml = true;
                }
                b'R' => {
                    if romu {
                        self.multiple(b'R');
                        return;
                    }
                    romu = true;
                }
                b'n' => {
                    if arab {
                        self.multiple(b'n');
                        return;
                    }
                    arab = true;
                }
                b'a' => {
                    if alpl {
                        self.multiple(b'A');
                        return;
                    }
                    alpl = true;
                }
                b'A' => {
                    if alpu {
                        self.multiple(b'A');
                        return;
                    }
                    alpu = true;
                }
                x => {
                    self.sty_skipline();
                    let m = fmt(
                        "Unknow type `%c' in page precedence specification.\n",
                        &[P::C(x)],
                    );
                    self.sty_error(&m);
                    return;
                }
            }
            i += 1;
        }
        if at(&pp, i) != 0 {
            self.sty_skipline();
            self.sty_error(b"Page precedence specification string too long.\n");
            return;
        }
        let mut last = i;
        if last == 0 {
            // An empty page_precedence: the C program goes on with an
            // uninitialised type[0] (undefined); leave the offsets as they
            // are.
            return;
        }
        match at(&pp, 0) {
            b'r' => {
                order[0] = ROMAN_LOWER_OFFSET;
                type_[0] = ROML as i32;
            }
            b'R' => {
                order[0] = ROMAN_UPPER_OFFSET;
                type_[0] = ROMU as i32;
            }
            b'n' => {
                order[0] = ARABIC_OFFSET;
                type_[0] = ARAB as i32;
            }
            b'a' => {
                order[0] = ALPHA_LOWER_OFFSET;
                type_[0] = ALPL as i32;
            }
            b'A' => {
                order[0] = ALPHA_LOWER_OFFSET;
                type_[0] = ALPU as i32;
            }
            _ => {}
        }

        i = 1;
        while i < last {
            match at(&pp, i) {
                b'r' => {
                    order[i] = order[i - 1] + ROMAN_LOWER_OFFSET;
                    type_[i] = ROML as i32;
                }
                b'R' => {
                    order[i] = order[i - 1] + ROMAN_UPPER_OFFSET;
                    type_[i] = ROMU as i32;
                }
                b'n' => {
                    order[i] = order[i - 1] + ARABIC_OFFSET;
                    type_[i] = ARAB as i32;
                }
                b'a' => {
                    order[i] = order[i - 1] + ALPHA_LOWER_OFFSET;
                    type_[i] = ALPL as i32;
                }
                b'A' => {
                    order[i] = order[i - 1] + ALPHA_LOWER_OFFSET;
                    type_[i] = ALPU as i32;
                }
                _ => {}
            }
            i += 1;
        }

        self.page_offset = [-1; PAGETYPE_MAX];
        self.page_offset[type_[0] as usize] = 0;
        for k in 1..last {
            self.page_offset[type_[k] as usize] = order[k - 1];
        }
        #[allow(clippy::needless_range_loop)] // k is also a type number
        for k in 0..PAGETYPE_MAX {
            if self.page_offset[k] == -1 {
                let t = type_[last - 1];
                order[last] = order[last - 1]
                    + match t as i16 {
                        ROML => ROMAN_LOWER_OFFSET,
                        ROMU => ROMAN_UPPER_OFFSET,
                        ARAB => ARABIC_OFFSET,
                        ALPL => ALPHA_LOWER_OFFSET,
                        _ => ALPHA_UPPER_OFFSET,
                    };
                type_[last] = k as i32;
                self.page_offset[k] = order[last];
                last += 1;
            }
        }
    }
}
