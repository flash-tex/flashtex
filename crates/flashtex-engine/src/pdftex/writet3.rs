//! `writet3.c` and `pkin.c`, ported: Type 3 fonts from PK bitmaps (or
//! `.pgc` files).
//!
//! pdfTeX writes a font as Type 3 when it has no map entry, or its map entry
//! names neither a PostScript name nor a font file (a bitmap font). It first
//! looks for `<font>.pgc` (a Type 3 font in pdfTeX's own `\pdfglyph` text
//! form, kpathsea's `misc fonts`); without one, it asks kpathsea for the PK
//! file at the font's resolution (`kpse_find_pk`, which runs mktexpk when
//! the file does not exist yet) and draws every used character's bitmap as
//! an inline image.
//!
//! The C file's globals are one [`T3`] per font written; `pkin.c`'s are one
//! [`Pk`] per PK file read.

use super::fonts::{Fonts, GlyphNames, NOTDEF};
use super::output::set_cur_file_name;
use crate::generated::Globals;
use std::collections::BTreeSet;

/// `T3_BUF_SIZE`: `append_eol`'s limit on a `.pgc` line.
const T3_BUF_SIZE: usize = 1024;

/// writet3.c's file-level state while one font is written.
struct T3 {
    /// `t3_file`: the `.pgc` or PK file, read whole.
    data: Vec<u8>,
    pos: usize,
    /// `feof(t3_file)`: a read has hit the end.
    eof: bool,
    /// `t3_line_array` up to `t3_line_ptr` (no terminating NUL).
    line: Vec<u8>,
    image_used: bool,
    char_procs: [i32; 256],
    /// `t3_char_widths`: C `float`s.
    char_widths: [f32; 256],
    glyph_num: i32,
    font_scale: f32,
    b: [i32; 4],
    is_pk_font: bool,
}

impl T3 {
    fn new() -> T3 {
        T3 {
            data: Vec::new(),
            pos: 0,
            eof: false,
            line: Vec::new(),
            image_used: false,
            char_procs: [0; 256],
            char_widths: [0.0; 256],
            glyph_num: 0,
            font_scale: 0.0,
            b: [0; 4],
            is_pk_font: false,
        }
    }

    /// `t3_getchar` (`xgetc`): the next byte, or -1 (`EOF`) at the end.
    fn getchar(&mut self) -> i32 {
        match self.data.get(self.pos) {
            Some(&c) => {
                self.pos += 1;
                c as i32
            }
            None => {
                self.eof = true;
                -1
            }
        }
    }

    /// `t3_prefix(s)`: `strncmp(t3_line_array, s, strlen(s)) == 0`, on the
    /// C view of the line (up to its first NUL).
    fn prefix(&self, s: &[u8]) -> bool {
        let end = self
            .line
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(self.line.len());
        self.line[..end].starts_with(s)
    }

    /// The C view of the line from byte `n` (`t3_line_array + n`).
    fn line_from(&self, n: usize) -> &[u8] {
        let end = self
            .line
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(self.line.len());
        self.line.get(n..end).unwrap_or(&[])
    }

    /// `update_bbox`.
    fn update_bbox(&mut self, llx: i32, lly: i32, urx: i32, ury: i32, is_first_glyph: bool) {
        if is_first_glyph {
            self.b = [llx, lly, urx, ury];
        } else {
            if llx < self.b[0] {
                self.b[0] = llx;
            }
            if lly < self.b[1] {
                self.b[1] = lly;
            }
            if urx > self.b[2] {
                self.b[2] = urx;
            }
            if ury > self.b[3] {
                self.b[3] = ury;
            }
        }
    }
}

/// `chardesc` (ptexlib.h): one character read from a PK file. `raster`
/// holds C `halfword`s, each a 16-bit word of the bitmap.
#[derive(Default)]
struct CharDesc {
    charcode: i32,
    cwidth: i32,
    cheight: i32,
    xoff: i32,
    yoff: i32,
    xescape: i32,
    raster: Vec<u16>,
}

/// pkin.c's file-level state. The PK file is `t3_file`, here `data`.
struct Pk<'a> {
    data: &'a [u8],
    pos: &'a mut usize,
    inputbyte: i32,
    flagbyte: i32,
    bitweight: i32,
    dynf: i32,
    repeatcount: i32,
    /// `realfunc`: `rest` instead of `pkpackednum`.
    realfunc_rest: bool,
    /// `pk_remainder` (a C `long`).
    pk_remainder: i64,
}

/// pkin.c's messages for a character whose runs overrun its box, and for
/// a file that ends too soon.
const TOO_MANY: &str = "error while unpacking; more bits than required";
const EOF: &str = "unexpected eof in pk file";

/// The raster words a run-length character may write however few bytes
/// encode them (a 16384 x 16384 glyph), and beyond that, the words per byte
/// read: far more than any PK file gftopk writes, far less than repeat and
/// huge counts can ask for.
const PK_FREE_WORDS: usize = 1 << 24;
const PK_WORDS_PER_BYTE: usize = 64;

/// `gpower`.
const GPOWER: [i32; 17] = [
    0, 1, 3, 7, 15, 31, 63, 127, 255, 511, 1023, 2047, 4095, 8191, 16383, 32767, 65535,
];

impl Pk<'_> {
    /// `pkbyte`.
    fn byte(&mut self, g: &mut Globals) -> i32 {
        match self.data.get(*self.pos) {
            Some(&c) => {
                *self.pos += 1;
                c as i32
            }
            None => g.pdftex_fail(EOF),
        }
    }

    /// `pkduo`.
    fn duo(&mut self, g: &mut Globals) -> i32 {
        let mut i = self.byte(g);
        if i > 127 {
            i -= 256;
        }
        i * 256 + self.byte(g)
    }

    /// `pktrio`.
    fn trio(&mut self, g: &mut Globals) -> i32 {
        let mut i = self.byte(g);
        if i > 127 {
            i -= 256;
        }
        i = i * 256 + self.byte(g);
        i * 256 + self.byte(g)
    }

    /// `pkquad`.
    fn quad(&mut self, g: &mut Globals) -> i32 {
        let mut i = self.byte(g);
        if i > 127 {
            i -= 256;
        }
        i = i.wrapping_mul(256).wrapping_add(self.byte(g));
        i = i.wrapping_mul(256).wrapping_add(self.byte(g));
        i.wrapping_mul(256).wrapping_add(self.byte(g))
    }

    /// `getnyb`.
    fn getnyb(&mut self, g: &mut Globals) -> i32 {
        if self.bitweight == 0 {
            self.bitweight = 16;
            self.inputbyte = self.byte(g);
            self.inputbyte >> 4
        } else {
            self.bitweight = 0;
            self.inputbyte & 15
        }
    }

    /// `getbit`.
    fn getbit(&mut self, g: &mut Globals) -> bool {
        self.bitweight >>= 1;
        if self.bitweight == 0 {
            self.inputbyte = self.byte(g);
            self.bitweight = 128;
        }
        self.inputbyte & self.bitweight != 0
    }

    /// `(*realfunc)()`.
    fn realfunc(&mut self, g: &mut Globals) -> i32 {
        if self.realfunc_rest {
            self.rest(g)
        } else {
            self.pkpackednum(g)
        }
    }

    /// `pkpackednum`. C recurses for every repeat-count nybble (14):
    /// `repeatcount = pkpackednum(); return (*realfunc)();`. Here the
    /// callers still waiting for their repeat count are counted in
    /// `pending`, and each tail call `(*realfunc)()` to `pkpackednum` loops,
    /// so a file of repeat counts ends at its end (`unexpected eof in pk
    /// file`) instead of in a stack overflow.
    fn pkpackednum(&mut self, g: &mut Globals) -> i32 {
        let mut pending: u64 = 0;
        'call: loop {
            let mut i = self.getnyb(g);
            let mut r = if i == 0 {
                let mut j;
                loop {
                    j = self.getnyb(g);
                    i += 1;
                    if j != 0 {
                        break;
                    }
                }
                if i > 3 {
                    // Damn, we got a huge count! We *fake* it by giving an
                    // artificially large repeat count.
                    self.handlehuge(g, i, j)
                } else {
                    while i > 0 {
                        j = j * 16 + self.getnyb(g);
                        i -= 1;
                    }
                    j - 15 + (13 - self.dynf) * 16 + self.dynf
                }
            } else if i <= self.dynf {
                i
            } else if i < 14 {
                (i - self.dynf - 1) * 16 + self.getnyb(g) + self.dynf + 1
            } else if i == 14 {
                // `repeatcount = pkpackednum();` comes first.
                pending += 1;
                continue 'call;
            } else {
                self.repeatcount = 1;
                // `return (*realfunc)();`
                if !self.realfunc_rest {
                    continue 'call;
                }
                self.rest(g)
            };
            // Return `r` to the callers still waiting for a repeat count.
            loop {
                if pending == 0 {
                    return r;
                }
                pending -= 1;
                self.repeatcount = r;
                if !self.realfunc_rest {
                    continue 'call;
                }
                r = self.rest(g);
            }
        }
    }

    /// `rest`.
    fn rest(&mut self, g: &mut Globals) -> i32 {
        if self.pk_remainder < 0 {
            // C negates a `long`; the most negative one has no negation
            // (undefined behaviour: pdfTeX then returns 0 forever).
            self.pk_remainder = match self.pk_remainder.checked_neg() {
                Some(r) => r,
                None => g.pdftex_fail(TOO_MANY),
            };
            0
        } else if self.pk_remainder > 0 {
            if self.pk_remainder > 4000 {
                self.pk_remainder = 4000 - self.pk_remainder;
                4000
            } else {
                let i = self.pk_remainder as i32;
                self.pk_remainder = 0;
                self.realfunc_rest = false;
                i
            }
        } else {
            g.pdftex_fail("shouldn't happen")
        }
    }

    /// `handlehuge`.
    fn handlehuge(&mut self, g: &mut Globals, mut i: i32, k: i32) -> i32 {
        let mut j = k as i64;
        while i != 0 {
            j = j.wrapping_shl(4).wrapping_add(self.getnyb(g) as i64);
            i -= 1;
        }
        self.pk_remainder = j
            .wrapping_sub(15)
            .wrapping_add((13 - self.dynf as i64) * 16 + self.dynf as i64);
        self.realfunc_rest = true;
        self.rest(g)
    }

    /// `unpack`. `rowsleft`, `hbit` and `wordwidth` are C `short`s.
    ///
    /// C's raster holds `n = 2 * cheight * wordwidth` words (at least 2).
    /// A malformed file can make C index past `gpower`, write or read past
    /// the raster, or spin: undefined behaviour, where pdfTeX usually
    /// crashes. Here every such file ends in a TeX error, checked before the
    /// work it would cause:
    ///
    /// - a run-length character writes at most `rowsleft * wordwidth` words
    ///   (C's `short`s): a write past that is the overshoot C reports as
    ///   `more bits than required`, or a write past `n`; `writepk` then
    ///   refuses to draw rows that were never decoded (C reads past its
    ///   raster there);
    /// - a bitmap character needs `cwidth * cheight` bits from the file, so
    ///   one the rest of the file cannot hold ends as pdfTeX's does, at its
    ///   end of file;
    /// - a run-length character writes at most [`PK_FREE_WORDS`] words, or
    ///   [`PK_WORDS_PER_BYTE`] words per byte of the file it has read,
    ///   whichever is more: repeat and huge counts cannot turn a few bytes
    ///   into gigabytes.
    ///
    /// Every character a well-formed PK file holds passes unchanged.
    fn unpack(&mut self, g: &mut Globals, cd: &mut CharDesc) {
        let wordwidth = (cd.cwidth.wrapping_add(15) / 16) as i16;
        let mut n = (2i64 * cd.cheight as i64 * wordwidth as i64) as i32;
        if n <= 0 {
            n = 2;
        }
        let start = *self.pos;
        let remaining = (self.data.len() - start) as i64;
        self.realfunc_rest = false;
        self.dynf = self.flagbyte / 16;
        let mut turnon = self.flagbyte & 8 != 0;
        let mut limit = n as usize;
        if self.dynf == 14 {
            if cd.cwidth > 0
                && cd.cheight > 0
                && cd.cwidth as i64 * cd.cheight as i64 > 8 * remaining
            {
                g.pdftex_fail(EOF);
            }
        } else {
            // C decodes `rowsleft` rows of `wordwidth` words (both `short`s,
            // as C truncates them); a write past those is an overshoot.
            let rows = (cd.cheight as i16).max(0) as usize;
            limit = limit.min(rows * wordwidth.max(0) as usize);
        }
        let (code, cwidth, cheight) = (cd.charcode, cd.cwidth, cd.cheight);
        let raster = &mut cd.raster;
        raster.clear();
        // `*raster++ = v`, bounded as above.
        let put = |g: &mut Globals, pk: &Self, raster: &mut Vec<u16>, v: i32| {
            let len = raster.len();
            if len >= limit {
                g.pdftex_fail(TOO_MANY);
            }
            if len >= PK_FREE_WORDS && len >= PK_WORDS_PER_BYTE * (*pk.pos - start) {
                g.pdftex_fail(&format!(
                    "character {} ({}x{}) encodes more than {PK_WORDS_PER_BYTE} raster words per byte",
                    code, cwidth, cheight
                ));
            }
            raster.push(v as u16);
        };
        // `gpower[k]`.
        let gpower = |g: &mut Globals, k: i32| -> i32 {
            match GPOWER.get(k as usize) {
                Some(&p) if k >= 0 => p,
                _ => g.pdftex_fail(TOO_MANY),
            }
        };
        if self.dynf == 14 {
            self.bitweight = 0;
            // With no columns a row reads and writes nothing.
            let rows = if cd.cwidth > 0 { cd.cheight } else { 0 };
            for _ in 1..=rows {
                let mut word = 0i32;
                let mut wordweight = 32768i32;
                for _ in 1..=cd.cwidth {
                    if self.getbit(g) {
                        word += wordweight;
                    }
                    wordweight >>= 1;
                    if wordweight == 0 {
                        put(g, self, raster, word);
                        word = 0;
                        wordweight = 32768;
                    }
                }
                if wordweight != 32768 {
                    put(g, self, raster, word);
                }
            }
        } else {
            let mut rowsleft = cd.cheight as i16;
            let mut hbit = cd.cwidth as i16;
            self.repeatcount = 0;
            let mut wordweight = 16i32;
            let mut word = 0i32;
            self.bitweight = 0;
            let ww = wordwidth as usize;
            while rowsleft > 0 {
                let mut count = self.realfunc(g);
                while count != 0 {
                    if count < wordweight && count < hbit as i32 {
                        if turnon {
                            word += gpower(g, wordweight) - gpower(g, wordweight - count);
                        }
                        hbit = (hbit as i32).wrapping_sub(count) as i16;
                        wordweight -= count;
                        count = 0;
                    } else if count >= hbit as i32 && hbit as i32 <= wordweight {
                        if turnon {
                            word += gpower(g, wordweight) - gpower(g, wordweight - hbit as i32);
                        }
                        put(g, self, raster, word);
                        // A row just ended, so the raster holds it whole.
                        if ww > 0 {
                            for _ in 1..=self.repeatcount {
                                for _ in 1..=ww {
                                    let Some(k) = raster.len().checked_sub(ww) else {
                                        g.pdftex_fail(TOO_MANY);
                                    };
                                    let v = raster[k] as i32;
                                    put(g, self, raster, v);
                                }
                            }
                        }
                        rowsleft =
                            (rowsleft as i32).wrapping_sub(self.repeatcount.wrapping_add(1)) as i16;
                        self.repeatcount = 0;
                        word = 0;
                        wordweight = 16;
                        count = count.wrapping_sub(hbit as i32);
                        hbit = cd.cwidth as i16;
                    } else {
                        if turnon {
                            word += gpower(g, wordweight);
                        }
                        put(g, self, raster, word);
                        word = 0;
                        count = count.wrapping_sub(wordweight);
                        hbit = (hbit as i32).wrapping_sub(wordweight) as i16;
                        wordweight = 16;
                    }
                }
                turnon = !turnon;
            }
            if rowsleft != 0 || hbit as i32 != cd.cwidth {
                g.pdftex_fail(TOO_MANY);
            }
        }
    }

    /// `readchar`: check the preamble if asked, read the next character
    /// definition into `cd`; false at the postamble.
    fn readchar(&mut self, g: &mut Globals, check_preamble: bool, cd: &mut CharDesc) -> bool {
        if check_preamble {
            if self.byte(g) != 247 {
                g.pdftex_fail("bad pk file, expected pre");
            }
            if self.byte(g) != 89 {
                g.pdftex_fail("bad version of pk file");
            }
            let mut i = self.byte(g); // creator of pkfile
            while i > 0 {
                self.byte(g);
                i -= 1;
            }
            self.quad(g); // design size
            self.quad(g); // checksum
            self.quad(g); // hppp
            self.quad(g); // vppp
        }
        // Now we skip to the desired character definition.
        loop {
            self.flagbyte = self.byte(g);
            if self.flagbyte == 245 {
                return false;
            }
            if self.flagbyte < 240 {
                let length: i64;
                match self.flagbyte & 7 {
                    0..=3 => {
                        length = ((self.flagbyte & 7) * 256 + self.byte(g) - 3) as i64;
                        cd.charcode = self.byte(g);
                        self.trio(g); // TFMwidth
                        cd.xescape = self.byte(g); // pixel width
                        cd.cwidth = self.byte(g);
                        cd.cheight = self.byte(g);
                        cd.xoff = self.byte(g);
                        cd.yoff = self.byte(g);
                        if cd.xoff > 127 {
                            cd.xoff -= 256;
                        }
                        if cd.yoff > 127 {
                            cd.yoff -= 256;
                        }
                    }
                    4..=6 => {
                        let mut l = (self.flagbyte & 3) as i64 * 65536 + self.byte(g) as i64 * 256;
                        l = l + self.byte(g) as i64 - 4;
                        length = l;
                        cd.charcode = self.byte(g);
                        self.trio(g); // TFMwidth
                        cd.xescape = self.duo(g); // pixelwidth
                        cd.cwidth = self.duo(g);
                        cd.cheight = self.duo(g);
                        cd.xoff = self.duo(g);
                        cd.yoff = self.duo(g);
                    }
                    _ => {
                        length = self.quad(g) as i64 - 9;
                        cd.charcode = self.quad(g);
                        self.quad(g); // TFMwidth
                        cd.xescape = self.quad(g); // pixelwidth
                        self.quad(g);
                        cd.cwidth = self.quad(g);
                        cd.cheight = self.quad(g);
                        cd.xoff = self.quad(g);
                        cd.yoff = self.quad(g);
                    }
                }
                if length <= 0 {
                    g.pdftex_fail(&format!("packet length ({}) too small", length as i32));
                }
                self.unpack(g, cd);
                return true;
            }
            let mut k: i32 = 0;
            match self.flagbyte {
                240..=243 => {
                    // The C cases fall through from 243 down to 240.
                    if self.flagbyte == 243 {
                        k = self.byte(g);
                        if k > 127 {
                            k -= 256;
                        }
                    }
                    if self.flagbyte >= 242 {
                        k = k.wrapping_mul(256).wrapping_add(self.byte(g));
                    }
                    if self.flagbyte >= 241 {
                        k = k.wrapping_mul(256).wrapping_add(self.byte(g));
                    }
                    k = k.wrapping_mul(256).wrapping_add(self.byte(g));
                    while k > 0 {
                        self.byte(g);
                        k -= 1;
                    }
                }
                244 => {
                    self.quad(g);
                }
                246 => {}
                f => g.pdftex_fail(&format!("unexpected command ({f})")),
            }
        }
    }
}

/// `kpse_magstep_fix` (kpathsea's magstep.c): the true resolution of `dpi`
/// at base resolution `bdpi`, a magstep of it if one is within 1.
fn magstep_fix(dpi: u32, bdpi: u32) -> u32 {
    fn magstep(mut n: i32, bdpi: i32) -> i32 {
        let mut neg = false;
        if n < 0 {
            neg = true;
            n = -n;
        }
        let mut t: f64;
        if n & 1 != 0 {
            n &= !1;
            t = 1.095445115;
        } else {
            t = 1.0;
        }
        while n > 8 {
            n -= 8;
            t *= 2.0736;
        }
        while n > 0 {
            n -= 2;
            t *= 1.2;
        }
        (0.5 + if neg {
            bdpi as f64 / t
        } else {
            bdpi as f64 * t
        }) as i32
    }
    const MAGSTEP_MAX: i32 = 40;
    let mut real_dpi: u32 = 0;
    let sign = if dpi < bdpi { -1 } else { 1 };
    let mut m = 0;
    while real_dpi == 0 && m < MAGSTEP_MAX {
        let mdpi = magstep(m * sign, bdpi as i32);
        if (mdpi - dpi as i32).abs() <= 1 {
            real_dpi = mdpi as u32;
        } else if (mdpi - dpi as i32) * sign > 0 {
            real_dpi = dpi;
        }
        m += 1;
    }
    if real_dpi != 0 {
        real_dpi
    } else {
        dpi
    }
}

/// `kpse_bitmap_tolerance` (kpathsea's tex-glyph.c): whether `dpi1` is
/// within `KPSE_BITMAP_TOLERANCE` of `dpi2`.
fn bitmap_tolerance(dpi1: f64, dpi2: f64) -> bool {
    let tolerance = (dpi2 / 500.0 + 1.0) as u32;
    let lower_bound: u32 = if ((dpi2 - tolerance as f64) as i32) < 0 {
        0
    } else {
        (dpi2 - tolerance as f64) as u32
    };
    let upper_bound = (dpi2 + tolerance as f64) as u32;
    lower_bound as f64 <= dpi1 && dpi1 <= upper_bound as f64
}

impl Globals {
    /// `t3_getline`: the next line of the `.pgc` file that is neither empty
    /// nor a comment, with blanks squeezed as `append_char_to_buf` does.
    fn t3_getline(&mut self, t3: &mut T3) {
        loop {
            t3.line.clear();
            let mut c = t3.getchar();
            while !t3.eof {
                // append_char_to_buf
                if c == 9 {
                    c = 32;
                }
                if c == 13 || c == -1 {
                    c = 10;
                }
                if c != b' ' as i32 || t3.line.last().is_some_and(|&p| p != 32) {
                    t3.line.push(c as u8);
                }
                if c == 10 {
                    break;
                }
                c = t3.getchar();
            }
            // append_eol
            if t3.line.len() + 2 > T3_BUF_SIZE {
                // `check_buf`'s message names `__FILE__` as TeX Live's
                // build compiles it.
                self.pdftex_fail(
                    "buffer overflow at file ../../../texk/web2c/pdftexdir/writet3.c, line 66",
                );
            }
            let n = t3.line.len();
            if n > 1 && t3.line[n - 1] != 10 {
                t3.line.push(10);
            }
            let n = t3.line.len();
            if n > 2 && t3.line[n - 2] == 32 {
                t3.line[n - 2] = 10;
                t3.line.pop();
            }
            if (t3.line.len() < 2 || t3.line.first() == Some(&b'%')) && !t3.eof {
                continue;
            }
            return;
        }
    }

    /// `t3_putline`.
    fn t3_putline(&mut self, t3: &T3) {
        for &c in &t3.line {
            self.pdf_out_byte(c);
        }
    }

    /// `t3_putchar` (`pdfout`, ptexmac.h).
    fn pdf_out_byte(&mut self, c: u8) {
        self.c_pdf_room(1);
        let p = self.pdf_ptr;
        self.pdf_buf_set(p, c as i32);
        self.pdf_ptr += 1;
    }

    /// `t3_write_glyph`: one `\pdfglyph` ... `\endglyph` of a `.pgc` file.
    fn t3_write_glyph(&mut self, t3: &mut T3, f: i32) {
        const BEGIN: &[u8] = b"\\pdfglyph";
        const END: &[u8] = b"\\endglyph";
        self.t3_getline(t3);
        if !t3.prefix(BEGIN) {
            return;
        }
        let (n, v) = super::cfmt::scan_ints8(t3.line_from(BEGIN.len() + 1));
        if n != 8 {
            let mut l = t3.line_from(0).to_vec();
            if l.last() == Some(&10) {
                l.pop();
            }
            self.pdftex_fail(&format!(
                "invalid glyph preamble: `{}'",
                String::from_utf8_lossy(&l)
            ));
        }
        let [glyph_index, width, _height, _depth, llx, lly, urx, ury] = v;
        if glyph_index < self.font_bc[f as usize] || glyph_index > self.font_ec[f as usize] {
            return;
        }
        if !self.pdf_char_marked(f, glyph_index) {
            while !t3.prefix(END) {
                if t3.eof {
                    self.pdftex_fail("unexpected end of file");
                }
                self.t3_getline(t3);
            }
            return;
        }
        t3.update_bbox(llx, lly, urx, ury, t3.glyph_num == 0);
        t3.glyph_num += 1;
        self.pdf_new_dict(0, 0, 0);
        t3.char_procs[glyph_index as usize] = self.obj_ptr;
        t3.char_widths[glyph_index as usize] = if width == 0 {
            (self.get_charwidth(f, glyph_index) as f32 / t3.font_scale)
                / self.pdf_font_size[f as usize] as f32
        } else {
            width as f32
        };
        self.pdf_begin_stream();
        self.t3_getline(t3);
        self.pdf_printf(
            format!(
                "{} 0 {llx} {lly} {urx} {ury} d1\nq\n",
                t3.char_widths[glyph_index as usize] as i32
            )
            .as_bytes(),
        );
        while !t3.prefix(END) {
            if t3.eof {
                self.pdftex_fail("unexpected end of file");
            }
            if t3.prefix(b"BI") {
                t3.image_used = true;
            }
            self.t3_putline(t3);
            self.t3_getline(t3);
        }
        self.pdf_puts(b"Q\n");
        self.pdf_end_stream();
    }

    /// `get_pk_font_scale`.
    fn get_pk_font_scale(&mut self, f: i32) -> i32 {
        let s = self.divide_scaled(
            self.pdf_font_size[f as usize],
            self.one_hundred_bp,
            self.fixed_decimal_digits + 2,
        );
        self.divide_scaled(self.pk_scale_factor, s, 0)
    }

    /// `pk_char_width`.
    fn pk_char_width(&mut self, f: i32, w: i32) -> i32 {
        let a = self.divide_scaled(w, self.pdf_font_size[f as usize], 7);
        let b = self.get_pk_font_scale(f);
        self.divide_scaled(a, b, 0)
    }

    /// `getpkcharwidth` (writet3.c): the width of a character of bitmap
    /// font `f` whose TFM width is `w`, as the Type 3 font's `/Widths`
    /// round it (pdftex.web's `adv_char_width`).
    pub fn get_pk_char_width(&mut self, f: i32, w: i32) -> i32 {
        let scale = self.get_pk_font_scale(f) as f64;
        let cw = self.pk_char_width(f, w) as f64;
        ((scale / 100000.0) * (cw / 100.0) * self.pdf_font_size[f as usize] as f64) as i32
    }

    /// `writepk`: the character procedures of font `f` from its PK file.
    fn writepk(&mut self, t3: &mut T3, f: i32) -> bool {
        // `round(fixedpkresolution * ((float) pdffontsize[f] / fontdsize[f]))`:
        // the product is a C `float`, rounded as a `double`.
        let res = self.fixed_pk_resolution;
        let ratio = self.pdf_font_size[f as usize] as f32 / self.font_dsize[f as usize] as f32;
        let x = (res as f32 * ratio) as f64;
        let dpi = magstep_fix(x.round() as u32, res as u32);
        let name = self.c_string(self.font_name[f as usize]);
        set_cur_file_name(Some(&name));
        let found = crate::system::find_pk(&name, dpi);
        let path = match found {
            Some(pk)
                if pk.name.as_bytes() == name.as_slice()
                    && bitmap_tolerance(pk.dpi as f32 as f64, dpi as f32 as f64) =>
            {
                pk.path
            }
            _ => self.pdftex_fail(&format!(
                "Font {} at {} not found",
                String::from_utf8_lossy(&name),
                dpi as i32
            )),
        };
        let data = match std::fs::read(&path) {
            Ok(d) => d,
            Err(e) => self.pdftex_fail(&format!("{path}: {e}")),
        };
        t3.image_used = true;
        t3.is_pk_font = true;
        self.tex_printf(format!(" <{path}").as_bytes());
        let mut cd = CharDesc {
            ..Default::default()
        };
        let mut pos = 0usize;
        let mut pk = Pk {
            data: &data,
            pos: &mut pos,
            inputbyte: 0,
            flagbyte: 0,
            bitweight: 0,
            dynf: 0,
            repeatcount: 0,
            realfunc_rest: false,
            pk_remainder: 0,
        };
        let mut check_preamble = true;
        while pk.readchar(self, check_preamble, &mut cd) {
            check_preamble = false;
            // A character code outside 0..255 (the long form allows one)
            // makes C read `pdfcharused` and `t3_char_widths` out of bounds;
            // pdfTeX 1.40.29 was observed to skip such a character, as
            // here.
            if !(0..=255).contains(&cd.charcode) || !self.pdf_char_marked(f, cd.charcode) {
                continue;
            }
            let c = cd.charcode as usize;
            let w = self.get_charwidth(f, cd.charcode);
            t3.char_widths[c] = self.pk_char_width(f, w) as f32;
            let is_null_glyph = if cd.cwidth < 1 || cd.cheight < 1 {
                cd.cwidth = (t3.char_widths[c] as f64 / 100.0).round() as i32;
                cd.xescape = cd.cwidth;
                cd.cheight = 1;
                cd.xoff = 0;
                cd.yoff = 0;
                true
            } else {
                false
            };
            // C `int` arithmetic; offsets from a malformed file wrap.
            let llx = cd.xoff.wrapping_neg();
            let lly = cd.yoff.wrapping_sub(cd.cheight).wrapping_add(1);
            let urx = cd.cwidth.wrapping_add(llx).wrapping_add(1);
            let ury = cd.cheight.wrapping_add(lly);
            t3.update_bbox(llx, lly, urx, ury, t3.glyph_num == 0);
            t3.glyph_num += 1;
            self.pdf_new_dict(0, 0, 0);
            t3.char_procs[c] = self.obj_ptr;
            self.pdf_begin_stream();
            self.pdf_print_real(t3.char_widths[c] as i32, 2);
            self.pdf_printf(format!(" 0 {llx} {lly} {urx} {ury} d1\n").as_bytes());
            if !is_null_glyph {
                self.pdf_printf(
                    format!("q\n{} 0 0 {} {llx} {lly} cm\nBI\n", cd.cwidth, cd.cheight).as_bytes(),
                );
                self.pdf_printf(format!("/W {}\n/H {}\n", cd.cwidth, cd.cheight).as_bytes());
                self.pdf_puts(b"/IM true\n/BPC 1\n/D [1 0]\nID ");
                let cw = cd.cwidth.wrapping_add(7) / 8;
                let rw = cd.cwidth.wrapping_add(15) / 16;
                // `unpack` wrote every row read here; C would read past its
                // raster otherwise.
                if cd.cheight as i64 * rw as i64 > cd.raster.len() as i64 {
                    self.pdftex_fail(TOO_MANY);
                }
                let word = |r: usize| cd.raster.get(r).copied().unwrap_or(0);
                let mut row = 0usize;
                for _ in 0..cd.cheight {
                    for _ in 0..rw - 1 {
                        self.pdf_out_byte((word(row) / 256) as u8);
                        self.pdf_out_byte((word(row) % 256) as u8);
                        row += 1;
                    }
                    self.pdf_out_byte((word(row) / 256) as u8);
                    if 2 * rw == cw {
                        self.pdf_out_byte((word(row) % 256) as u8);
                    }
                    row += 1;
                }
                self.pdf_puts(b"\nEI\nQ\n");
            }
            self.pdf_end_stream();
        }
        set_cur_file_name(None);
        true
    }

    /// `writet3` (writet3.c): the Type 3 font dictionary of font `f`, PDF
    /// object `objnum`, with map entry `fm` (if it has one).
    pub fn writet3(&mut self, st: &mut Fonts, fm: Option<usize>, objnum: i32, f: i32) {
        let mut t3 = T3::new();
        let encname = fm.and_then(|id| st.map.fms[id].as_ref().and_then(|e| e.encname.clone()));
        let tfm_name = fm
            .and_then(|id| st.map.fms[id].as_ref().map(|e| e.tfm_name.clone()))
            .unwrap_or_default();
        let fe = match &encname {
            Some(e) => self.get_fe_entry(st, e),
            None => None,
        };
        if let (Some(fe), Some(e)) = (fe, &encname) {
            let mut names = std::mem::take(&mut st.enc.fes[fe].glyph_names);
            self.remove_duplicate_glyph_names(&mut names, e);
            st.enc.fes[fe].glyph_names = names;
        }
        let glyph_names: Option<GlyphNames> = fe.map(|fe| st.enc.fes[fe].glyph_names.clone());
        let pgc = self.make_tex_string(b".pgc");
        let null = self.null_str();
        self.pack_file_name(self.font_name[f as usize], null, pgc);
        let s = self.make_name_string();
        let pgc_name = self.c_string(s);
        set_cur_file_name(Some(&pgc_name));
        t3.is_pk_font = false;
        match self.open_misc_font_input() {
            None => {
                if !self.writepk(&mut t3, f) {
                    set_cur_file_name(None);
                    return;
                }
            }
            Some(path) => {
                t3.data = match std::fs::read(&path) {
                    Ok(d) => d,
                    // C reads the file it has just opened; a read that fails
                    // here must not leave the font object unwritten.
                    Err(e) => self.pdftex_fail(&format!("cannot read `{path}': {e}")),
                };
                self.tex_printf(format!("<{path}").as_bytes());
                self.t3_getline(&mut t3);
                const SCALE: &[u8] = b"\\pdffontscale";
                let ok = t3.prefix(SCALE)
                    && match super::cfmt::scan_float(t3.line_from(SCALE.len() + 1)) {
                        Some(s) => {
                            t3.font_scale = s;
                            s > 0.0 && s <= 1000.0
                        }
                        None => false,
                    };
                if !ok {
                    self.pdftex_fail("missing or invalid font scale");
                }
                while !t3.eof {
                    self.t3_write_glyph(&mut t3, f);
                }
            }
        }
        // write_font_dict:
        let (bc, ec) = (self.font_bc[f as usize], self.font_ec[f as usize]);
        let mut i = bc;
        while i <= ec {
            if self.pdf_char_marked(f, i) {
                break;
            }
            i += 1;
        }
        let first_char = i;
        let mut i = ec;
        while i > first_char {
            if self.pdf_char_marked(f, i) {
                break;
            }
            i -= 1;
        }
        let last_char = i;
        // write ToUnicode entry if we can
        let tounicode_objnum = match (&glyph_names, fe) {
            (Some(names), Some(fe))
                if self.fixed_gen_tounicode > 0
                    && !self.pdf_font_nobuiltin_tounicode[f as usize] =>
            {
                let enc = st.enc.fes[fe].name.clone();
                self.write_tounicode(st, names, &tfm_name, Some(&enc))
            }
            _ => 0,
        };
        self.pdf_begin_dict(objnum, 1); // Type 3 font dictionary
        self.pdf_puts(b"/Type /Font\n/Subtype /Type3\n");
        self.pdf_printf(format!("/Name /F{f}\n").as_bytes());
        let attr = self.pdf_font_attr[f as usize];
        if attr != self.null_str() {
            self.pdf_print(attr);
            self.pdf_puts(b"\n");
        }
        if t3.is_pk_font {
            let pk_font_scale = self.get_pk_font_scale(f);
            self.pdf_puts(b"/FontMatrix [");
            self.pdf_print_real(pk_font_scale, 5);
            self.pdf_puts(b" 0 0 ");
            self.pdf_print_real(pk_font_scale, 5);
            self.pdf_puts(b" 0 0]\n");
        } else {
            let s = super::cfmt::fmt_g(t3.font_scale as f64);
            let mut l = b"/FontMatrix [".to_vec();
            l.extend_from_slice(&s);
            l.extend_from_slice(b" 0 0 ");
            l.extend_from_slice(&s);
            l.extend_from_slice(b" 0 0]\n");
            self.pdf_printf(&l);
        }
        let [b0, b1, b2, b3] = t3.b;
        self.pdf_printf(format!("/FontBBox [ {b0} {b1} {b2} {b3} ]\n").as_bytes());
        self.pdf_printf(
            format!(
                "/Resources << /ProcSet [ /PDF {}] >>\n",
                if t3.image_used { "/ImageB " } else { "" }
            )
            .as_bytes(),
        );
        self.pdf_printf(format!("/FirstChar {first_char}\n/LastChar {last_char}\n").as_bytes());
        let wptr = self.pdf_new_objnum();
        let eptr = self.pdf_new_objnum();
        let cptr = self.pdf_new_objnum();
        self.pdf_printf(
            format!("/Widths {wptr} 0 R\n/Encoding {eptr} 0 R\n/CharProcs {cptr} 0 R\n").as_bytes(),
        );
        if tounicode_objnum != 0 {
            self.pdf_printf(format!("/ToUnicode {tounicode_objnum} 0 R\n").as_bytes());
        }
        self.pdf_end_dict();
        self.pdf_begin_obj(wptr, 1); // chars width array
        self.pdf_puts(b"[");
        for i in first_char..=last_char {
            if t3.is_pk_font {
                self.pdf_print_real(t3.char_widths[i as usize] as i32, 2);
                self.pdf_puts(b" ");
            } else {
                self.pdf_printf(format!("{} ", t3.char_widths[i as usize] as i32).as_bytes());
            }
        }
        self.pdf_puts(b"]\n");
        self.pdf_end_obj();
        // The glyph name of slot `i`, or `/a<i>`.
        let glyph = |i: i32| -> Vec<u8> {
            match glyph_names.as_ref().map(|g| &g[i as usize]) {
                Some(n) if n.as_slice() != NOTDEF => n.clone(),
                _ => format!("a{i}").into_bytes(),
            }
        };
        self.pdf_begin_dict(eptr, 1); // encoding dictionary
        self.pdf_printf(format!("/Type /Encoding\n/Differences [{first_char}").as_bytes());
        let mut is_notdef;
        if t3.char_procs[first_char as usize] == 0 {
            self.pdf_printf(b"/.notdef");
            is_notdef = true;
        } else {
            let mut s = b"/".to_vec();
            s.extend_from_slice(&glyph(first_char));
            self.pdf_printf(&s);
            is_notdef = false;
        }
        for i in first_char + 1..=last_char {
            if t3.char_procs[i as usize] == 0 {
                if !is_notdef {
                    self.pdf_printf(format!(" {i}/.notdef").as_bytes());
                    is_notdef = true;
                }
            } else {
                if is_notdef {
                    self.pdf_printf(format!(" {i}").as_bytes());
                    is_notdef = false;
                }
                let mut s = b"/".to_vec();
                s.extend_from_slice(&glyph(i));
                self.pdf_printf(&s);
            }
        }
        self.pdf_puts(b"]\n");
        self.pdf_end_dict();
        self.pdf_begin_dict(cptr, 1); // CharProcs dictionary
        for i in first_char..=last_char {
            if t3.char_procs[i as usize] != 0 {
                let mut s = b"/".to_vec();
                s.extend_from_slice(&glyph(i));
                s.extend_from_slice(format!(" {} 0 R\n", t3.char_procs[i as usize]).as_bytes());
                self.pdf_printf(&s);
            }
        }
        self.pdf_end_dict();
        self.tex_printf(b">");
        set_cur_file_name(None);
    }

    /// `remove_duplicate_glyph_names`: a name that occurs twice makes the
    /// PDF invalid, so every later occurrence becomes `.notdef`, with a
    /// warning.
    fn remove_duplicate_glyph_names(&mut self, g: &mut GlyphNames, encname: &[u8]) {
        let mut seen: BTreeSet<Vec<u8>> = BTreeSet::new();
        for (i, name) in g.iter_mut().enumerate().take(256) {
            if name.as_slice() == NOTDEF {
                continue;
            }
            if !seen.contains(name) {
                seen.insert(name.clone());
            } else {
                let mut msg = encname.to_vec();
                msg.extend_from_slice(
                    format!(": duplicate glyph name at position {i}: ").as_bytes(),
                );
                msg.extend_from_slice(name);
                self.pdftex_warn_bytes(&msg);
                *name = NOTDEF.to_vec();
            }
        }
    }

    /// `kpse_init_prog` and `kpse_set_program_enabled` for PK fonts
    /// (pdftex.web's `@<Initialize variables for \.{PDF} output@>`): the
    /// resolution and mode mktexpk makes PK files at, and mktexpk enabled.
    pub fn pk_init(&mut self, resolution: i32, pk_mode: i32) {
        let mode = (pk_mode != 0).then(|| self.c_string(pk_mode));
        crate::system::pk_init(resolution as u32, mode.as_deref());
    }
}
