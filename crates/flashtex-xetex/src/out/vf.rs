//! Virtual fonts, as xdvipdfmx reads and runs them: a port of TeX Live
//! 2026's dvipdfm-x `vf.c` (the VF file and its packets), the TFM half of
//! `tfm.c` (the fix-word widths, heights and depths a packet's characters
//! advance by) and `numbers.c`'s `sqxfw` (the scaling of every fix word).
//!
//! A TFM font of the XDV stream with no font map entry is looked for as a
//! virtual font (`dvi_locate_font`, [`super::fonts::Fonts::locate`]). Each
//! of its characters is then a packet of DVI commands, run at the
//! character's place ([`super::page::Builder`]'s `vf_set_char`) in the
//! fonts the VF file names, which are located the same way: through the
//! font map to a Type 1 program, or to another virtual font.
//!
//! This module is the file formats and the arithmetic only, with no state
//! of a run, so that it is tested on bytes alone. Where xdvipdfmx stops
//! with a fatal `ERROR`, these functions return an `Err` with its message.
//!
//! Not ported: Omega's OFM metrics and OVF files (`tfm.c`'s `read_ofm`,
//! the `kpse_ovf_format` search) and pTeX's JFM metrics; XeTeX loads
//! neither as a TFM font.

use std::collections::HashMap;
use std::sync::Arc;

/// `numbers.c`'s `sqxfw`: a scaled quantity times a fix word (2^-20),
/// rounded as xdvipdfmx rounds it. The product's low 16 bits are dropped
/// before the rounding, so this is not the nearest integer in every case,
/// and not TeX's own (truncating) rule either.
pub fn sqxfw(sq: i32, fw: i32) -> i32 {
    let mut negative = false;
    let (mut sq, mut fw) = (sq, fw);
    if sq < 0 {
        negative = !negative;
        sq = sq.wrapping_neg();
    }
    if fw < 0 {
        negative = !negative;
        fw = fw.wrapping_neg();
    }
    let a = (sq as u32) >> 16;
    let b = (sq as u32) & 0xffff;
    let c = (fw as u32) >> 16;
    let d = (fw as u32) & 0xffff;
    let (ad, bd, bc, ac) = (
        a.wrapping_mul(d),
        b.wrapping_mul(d),
        b.wrapping_mul(c),
        a.wrapping_mul(c),
    );
    let e = bd >> 16;
    let f = ad >> 16;
    let g = ad & 0xffff;
    let h = bc >> 16;
    let i = bc & 0xffff;
    let j = ac >> 16;
    let k = ac & 0xffff;
    let mut result = (e + g + i + (1 << 3)) >> 4; // 1 << 3 is for rounding
    result = result.wrapping_add((f + h + k) << 12);
    result = result.wrapping_add(j << 28);
    let result = result as i32;
    if negative {
        result.wrapping_neg()
    } else {
        result
    }
}

/// A TFM file's character dimensions as fix words (`tfm.c`'s
/// `font_metric` of a TFM file): 256 of each, 0 outside `first..=last`.
#[derive(Clone, Debug, PartialEq)]
pub struct Tfm {
    pub first: i32,
    pub last: i32,
    widths: Vec<i32>,
    heights: Vec<i32>,
    depths: Vec<i32>,
}

/// A big-endian reader that fails where `numbers.c`'s `get_unsigned_byte`
/// stops the program.
struct Reader<'a> {
    b: &'a [u8],
    i: usize,
}

const EOF: &str = "File ended prematurely";

impl<'a> Reader<'a> {
    fn byte(&mut self) -> Result<u8, String> {
        let c = *self.b.get(self.i).ok_or(EOF)?;
        self.i += 1;
        Ok(c)
    }

    fn pair(&mut self) -> Result<u32, String> {
        Ok((self.byte()? as u32) << 8 | self.byte()? as u32)
    }

    fn unsigned_quad(&mut self) -> Result<u32, String> {
        let mut v = 0u32;
        for _ in 0..4 {
            v = (v << 8) | self.byte()? as u32;
        }
        Ok(v)
    }

    fn signed_quad(&mut self) -> Result<i32, String> {
        Ok(self.unsigned_quad()? as i32)
    }

    /// `get_positive_quad`.
    fn positive_quad(&mut self, kind: &str, name: &str) -> Result<u32, String> {
        let v = self.signed_quad()?;
        if v < 0 {
            return Err(format!("Bad {kind}: negative {name}: {v}"));
        }
        Ok(v as u32)
    }

    /// `get_unsigned_num`: `num` + 1 bytes, unsigned except that four
    /// bytes are a signed quad.
    fn unsigned_num(&mut self, num: u8) -> Result<i32, String> {
        let mut val = self.byte()? as i32;
        if num == 3 && val > 0x7f {
            val -= 0x100;
        }
        for _ in 0..num.min(3) {
            val = (val << 8) | self.byte()? as i32;
        }
        Ok(val)
    }

    fn skip(&mut self, n: usize) -> Result<(), String> {
        for _ in 0..n {
            self.byte()?;
        }
        Ok(())
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let s = self.b.get(self.i..self.i + n).ok_or(EOF)?;
        self.i += n;
        Ok(s)
    }
}

impl Tfm {
    /// `read_tfm` (with `tfm_get_sizes`, `tfm_check_size` and
    /// `tfm_unpack_arrays`) on the bytes of a TFM file.
    pub fn parse(data: &[u8]) -> Result<Tfm, String> {
        if data.len() < 24 {
            return Err("TFM/OFM file too small to be a valid file.".into());
        }
        let mut r = Reader { b: data, i: 0 };
        let wlenfile = r.pair()?;
        if wlenfile == 9 || wlenfile == 11 {
            // tfm.c: "we always assume that TFMs starting with 00 09 or
            // 00 0B is JFM"
            return Err("a JFM (pTeX) metric file".into());
        }
        let wlenheader = r.pair()?;
        let bc = r.pair()?;
        let ec = r.pair()?;
        if ec < bc {
            return Err(format!("TFM file error: ec({ec}) < bc({bc}) ???"));
        }
        let nwidths = r.pair()?;
        let nheights = r.pair()?;
        let ndepths = r.pair()?;
        let nitcor = r.pair()?;
        let nlig = r.pair()?;
        let nkern = r.pair()?;
        let nextens = r.pair()?;
        let nfonparm = r.pair()?;
        if (data.len() as u64) < wlenfile as u64 * 4 {
            return Err("Can't proceed...".into());
        }
        let expected = 6
            + (ec - bc + 1)
            + wlenheader
            + nwidths
            + nheights
            + ndepths
            + nitcor
            + nlig
            + nkern
            + nextens
            + nfonparm;
        // a file that says another size than its tables make is read
        // ("Proceeding nervously...") when it is longer than they need
        if expected != wlenfile && data.len() as u64 <= expected as u64 * 4 {
            return Err("Can't proceed...".into());
        }
        r.skip(4 * wlenheader as usize)?;
        let n = (ec - bc + 1) as usize;
        let mut char_info = Vec::with_capacity(n);
        for _ in 0..n {
            char_info.push(r.unsigned_quad()?);
        }
        let mut table =
            |n: u32| -> Result<Vec<i32>, String> { (0..n).map(|_| r.signed_quad()).collect() };
        let width = table(nwidths)?;
        let height = table(nheights)?;
        let depth = table(ndepths)?;
        let mut t = Tfm {
            first: bc as i32,
            last: ec as i32,
            widths: vec![0; 256],
            heights: vec![0; 256],
            depths: vec![0; 256],
        };
        // (a TFM file's `ec` is at most 255; one that says more has no
        // place in tfm.c's arrays of 256)
        for (i, info) in char_info.iter().enumerate() {
            let c = bc as usize + i;
            if c >= 256 {
                break;
            }
            let at = |t: &[i32], i: u32| t.get(i as usize).copied().unwrap_or(0);
            t.widths[c] = at(&width, info >> 24);
            t.heights[c] = at(&height, (info >> 20) & 0xf);
            t.depths[c] = at(&depth, (info >> 16) & 0xf);
        }
        Ok(t)
    }

    /// The index `tfm_get_fw_width` and its siblings read at: the
    /// character's own within `first..=last`, and 0 above `last` (what
    /// pTeX's metrics want of every TFM file); a character below `first`
    /// is tfm.c's fatal "Invalid char".
    fn index(&self, ch: i32) -> Option<usize> {
        if ch >= self.first && ch <= self.last {
            Some(ch as usize)
        } else if ch > self.last {
            Some(0)
        } else {
            None
        }
    }

    /// `tfm_get_fw_width`.
    pub fn fw_width(&self, ch: i32) -> Option<i32> {
        self.index(ch)
            .map(|i| self.widths.get(i).copied().unwrap_or(0))
    }

    /// `tfm_get_fw_height`.
    pub fn fw_height(&self, ch: i32) -> Option<i32> {
        self.index(ch)
            .map(|i| self.heights.get(i).copied().unwrap_or(0))
    }

    /// `tfm_get_fw_depth`.
    pub fn fw_depth(&self, ch: i32) -> Option<i32> {
        self.index(ch)
            .map(|i| self.depths.get(i).copied().unwrap_or(0))
    }
}

/// A font a VF file defines (`vf.c`'s `font_def`, less what locating it
/// gives).
#[derive(Clone, Debug, PartialEq)]
pub struct VfFontDef {
    /// The number the packets select it by.
    pub font_id: i32,
    pub checksum: u32,
    /// Its size as a fix word, relative to the virtual font's size.
    pub size: u32,
    pub design_size: u32,
    pub name: Vec<u8>,
}

/// A VF file as `vf_locate_font` reads it (`read_header`,
/// `process_vf_file`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Vf {
    /// A fix word of points.
    pub design_size: u32,
    /// In the file's order: the first is a packet's font at its start.
    pub fonts: Vec<VfFontDef>,
    /// Each character's packet. One without a packet (or with an empty
    /// one) is `vf_set_char`'s "nonexistent character".
    packets: HashMap<u32, Arc<[u8]>>,
}

/// DVI's opcodes, as a packet uses them (`dvicodes.h`).
mod op {
    pub const SET_CHAR_127: u8 = 127;
    pub const SET1: u8 = 128;
    pub const SET3: u8 = 130;
    pub const SET4: u8 = 131;
    pub const SET_RULE: u8 = 132;
    pub const PUT1: u8 = 133;
    pub const PUT3: u8 = 135;
    pub const PUT4: u8 = 136;
    pub const PUT_RULE: u8 = 137;
    pub const NOP: u8 = 138;
    pub const PUSH: u8 = 141;
    pub const POP: u8 = 142;
    pub const RIGHT1: u8 = 143;
    pub const RIGHT4: u8 = 146;
    pub const W0: u8 = 147;
    pub const W1: u8 = 148;
    pub const W4: u8 = 151;
    pub const X0: u8 = 152;
    pub const X1: u8 = 153;
    pub const X4: u8 = 156;
    pub const DOWN1: u8 = 157;
    pub const DOWN4: u8 = 160;
    pub const Y0: u8 = 161;
    pub const Y1: u8 = 162;
    pub const Y4: u8 = 165;
    pub const Z0: u8 = 166;
    pub const Z1: u8 = 167;
    pub const Z4: u8 = 170;
    pub const FNT_NUM_0: u8 = 171;
    pub const FNT_NUM_63: u8 = 234;
    pub const FNT1: u8 = 235;
    pub const FNT4: u8 = 238;
    pub const XXX1: u8 = 239;
    pub const XXX4: u8 = 242;
    pub const FNT_DEF1: u8 = 243;
    pub const FNT_DEF4: u8 = 246;
    pub const PRE: u8 = 247;
    pub const POST: u8 = 248;
    pub const PTEXDIR: u8 = 255;
}

const VF_ID: u8 = 202;
/// Characters from here on are kept in an index, in increasing order.
const CHAR_INDEX_MIN: u32 = 0x40000;

impl Vf {
    /// The bytes of a VF file. An `Err` is where xdvipdfmx stops with a
    /// fatal error (a file that ends early, a negative size or length, a
    /// character of more than 24 bits, large characters out of order).
    pub fn parse(data: &[u8]) -> Result<Vf, String> {
        let mut r = Reader { b: data, i: 0 };
        let mut vf = Vf::default();
        // read_header: the usual signature, else "VF file may be corrupt"
        // and the file is read on from there
        if r.byte()? == op::PRE && r.byte()? == VF_ID {
            let k = r.byte()? as usize;
            r.skip(k)?; // the comment
            r.skip(4)?; // the checksum
            vf.design_size = r.positive_quad("VF", "design_size")?;
        }
        let mut last_big: Option<u32> = None;
        let mut def = |vf: &mut Vf, r: &mut Reader, len: u32, ch: u32| -> Result<(), String> {
            if ch >= CHAR_INDEX_MIN {
                if last_big.is_some_and(|l| l >= ch) {
                    return Err("Unexpected character code in vf file".into());
                }
                last_big = Some(ch);
            }
            if len > 0 {
                let pkt = r
                    .take(len as usize)
                    .map_err(|_| "VF file ended prematurely.")?;
                vf.packets.insert(ch, Arc::from(pkt));
            } else {
                // a length of 0 leaves no bytes to run, whatever an
                // earlier definition of the character gave
                vf.packets.remove(&ch);
            }
            Ok(())
        };
        // process_vf_file
        loop {
            let code = r.byte()?;
            match code {
                op::FNT_DEF1..=op::FNT_DEF4 => {
                    let font_id = r.unsigned_num(code - op::FNT_DEF1)?;
                    let checksum = r.unsigned_quad()?;
                    let size = r.positive_quad("VF", "font_size")?;
                    let design_size = r.positive_quad("VF", "font_design_size")?;
                    let dir = r.byte()? as usize;
                    let name = r.byte()? as usize;
                    r.take(dir)?;
                    let name = r.take(name)?.to_vec();
                    vf.fonts.push(VfFontDef {
                        font_id,
                        checksum,
                        size,
                        design_size,
                        name,
                    });
                }
                0..=241 => {
                    // a short packet: the code is its length
                    let ch = r.byte()? as u32;
                    r.skip(3)?; // the TFM width
                    def(&mut vf, &mut r, code as u32, ch)?;
                }
                242 => {
                    let len = r.positive_quad("VF", "pkt_len")?;
                    let ch = r.unsigned_quad()?;
                    r.skip(4)?;
                    if ch >= 0x100_0000 {
                        return Err("Long character (>24 bits) in VF file.".into());
                    }
                    def(&mut vf, &mut r, len, ch)?;
                }
                op::POST => break,
                // so does any other code ("Quitting on code=")
                _ => break,
            }
        }
        Ok(vf)
    }

    /// Character `ch`'s packet, if it has one with bytes to run.
    pub fn packet(&self, ch: i32) -> Option<Arc<[u8]>> {
        let ch = u32::try_from(ch).ok()?;
        self.packets.get(&ch).cloned()
    }
}

/// One command of a packet, its numbers as the packet has them: lengths
/// are fix words still to be scaled by the virtual font's size
/// ([`sqxfw`]).
#[derive(Clone, Debug, PartialEq)]
pub enum Cmd<'a> {
    Set(i32),
    Put(i32),
    SetRule {
        height: i32,
        width: i32,
    },
    PutRule {
        height: i32,
        width: i32,
    },
    Nop,
    Push,
    Pop,
    Right(i32),
    W0,
    W(i32),
    X0,
    X(i32),
    Down(i32),
    Y0,
    Y(i32),
    Z0,
    Z(i32),
    /// Select the font the VF file defines with this number.
    Fnt(i32),
    Special(&'a [u8]),
    /// A special whose length reads as negative: a warning, and nothing
    /// is skipped.
    BadSpecial(i32),
    /// pTeX's direction change.
    Dir(u8),
}

/// The commands of a packet, in order (`vf_set_char`'s loop).
pub struct Packet<'a> {
    b: &'a [u8],
    i: usize,
}

const PREMATURE: &str = "Premature end of DVI byte stream in VF font";

impl<'a> Packet<'a> {
    pub fn new(b: &'a [u8]) -> Packet<'a> {
        Packet { b, i: 0 }
    }

    fn left(&self) -> usize {
        self.b.len() - self.i
    }

    fn next_byte(&mut self) -> i32 {
        let c = self.b[self.i];
        self.i += 1;
        c as i32
    }

    /// `get_pkt_signed_num`: `num` + 1 bytes.
    fn signed_num(&mut self, num: u8) -> Result<i32, String> {
        if self.left() <= num as usize {
            return Err(PREMATURE.into());
        }
        let mut val = self.next_byte();
        if val > 0x7f {
            val -= 0x100;
        }
        for _ in 0..num.min(3) {
            val = (val << 8) | self.next_byte();
        }
        Ok(val)
    }

    /// `get_pkt_unsigned_num`: `num` + 1 bytes, unsigned except that four
    /// bytes are a signed quad.
    fn unsigned_num(&mut self, num: u8) -> Result<i32, String> {
        if self.left() <= num as usize {
            return Err(PREMATURE.into());
        }
        let mut val = self.next_byte();
        if num == 3 && val > 0x7f {
            val -= 0x100;
        }
        for _ in 0..num.min(3) {
            val = (val << 8) | self.next_byte();
        }
        Ok(val)
    }

    /// The next command; `None` at the packet's end. An `Err` is a fatal
    /// error of xdvipdfmx's.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<Result<Cmd<'a>, String>> {
        if self.i >= self.b.len() {
            return None;
        }
        Some(self.command())
    }

    fn command(&mut self) -> Result<Cmd<'a>, String> {
        use op::*;
        let opcode = self.b[self.i];
        self.i += 1;
        Ok(match opcode {
            SET1..=SET3 => Cmd::Set(self.unsigned_num(opcode - SET1)?),
            PUT1..=PUT3 => Cmd::Put(self.unsigned_num(opcode - PUT1)?),
            SET4 | PUT4 => {
                return Err("Multibyte (>24 bits) character in VF packet.".into());
            }
            SET_RULE | PUT_RULE => {
                let height = self.signed_num(3)?;
                let width = self.signed_num(3)?;
                if opcode == SET_RULE {
                    Cmd::SetRule { height, width }
                } else {
                    Cmd::PutRule { height, width }
                }
            }
            NOP => Cmd::Nop,
            PUSH => Cmd::Push,
            POP => Cmd::Pop,
            RIGHT1..=RIGHT4 => Cmd::Right(self.signed_num(opcode - RIGHT1)?),
            W0 => Cmd::W0,
            W1..=W4 => Cmd::W(self.signed_num(opcode - W1)?),
            X0 => Cmd::X0,
            X1..=X4 => Cmd::X(self.signed_num(opcode - X1)?),
            DOWN1..=DOWN4 => Cmd::Down(self.signed_num(opcode - DOWN1)?),
            Y0 => Cmd::Y0,
            Y1..=Y4 => Cmd::Y(self.signed_num(opcode - Y1)?),
            Z0 => Cmd::Z0,
            Z1..=Z4 => Cmd::Z(self.signed_num(opcode - Z1)?),
            // (signed, where the file's own `fnt_def1` number is not:
            // font 128 and up, defined by one byte, cannot be selected by
            // `fnt1`, as in vf.c)
            FNT1..=FNT4 => Cmd::Fnt(self.signed_num(opcode - FNT1)?),
            XXX1..=XXX4 => {
                let len = self.unsigned_num(opcode - XXX1)?;
                if len < 0 {
                    Cmd::BadSpecial(len)
                } else {
                    // vf_xxx
                    let len = len as usize;
                    if self.left() < len {
                        return Err(format!("{PREMATURE}."));
                    }
                    let s = &self.b[self.i..self.i + len];
                    self.i += len;
                    Cmd::Special(s)
                }
            }
            PTEXDIR => {
                // unsigned_byte
                if self.left() == 0 {
                    return Err(PREMATURE.into());
                }
                Cmd::Dir(self.next_byte() as u8)
            }
            0..=SET_CHAR_127 => Cmd::Set(opcode as i32),
            FNT_NUM_0..=FNT_NUM_63 => Cmd::Fnt((opcode - FNT_NUM_0) as i32),
            _ => return Err(format!("Unexpected opcode in vf file: {opcode}")),
        })
    }
}

/// What `vf_xxx` does with a special's bytes: one that says `Warning:`
/// after its leading spaces is a message of the virtual font's (shown
/// only when xdvipdfmx is verbose), and any other is the page's special.
pub fn is_warning_special(s: &[u8]) -> bool {
    let p = s.iter().position(|&c| c != b' ').unwrap_or(s.len());
    s[p..].starts_with(b"Warning:")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `sqxfw` against the values TeX Live 2026's xdvipdfmx computes: its
    /// own `sqxfw` compiled from numbers.c (the PC, x86_64-linux), on
    /// these arguments.
    #[test]
    fn sqxfw_is_numbers_c() {
        for &(sq, fw, want) in SQXFW_ORACLE {
            assert_eq!(sqxfw(sq, fw), want, "sqxfw({sq}, {fw})");
        }
    }

    include!("testdata/sqxfw_oracle.rs");

    fn tfm_bytes(bc: u16, ec: u16, infos: &[u32], w: &[i32], h: &[i32], d: &[i32]) -> Vec<u8> {
        let lh = 2u16;
        let lf = 6 + lh + (ec - bc + 1) + w.len() as u16 + h.len() as u16 + d.len() as u16;
        let mut b = Vec::new();
        for v in [
            lf,
            lh,
            bc,
            ec,
            w.len() as u16,
            h.len() as u16,
            d.len() as u16,
            0,
            0,
            0,
            0,
            0,
        ] {
            b.extend_from_slice(&v.to_be_bytes());
        }
        b.extend_from_slice(&[0; 8]); // header: checksum, design size
        for i in infos {
            b.extend_from_slice(&i.to_be_bytes());
        }
        for t in [w, h, d] {
            for v in t {
                b.extend_from_slice(&v.to_be_bytes());
            }
        }
        b
    }

    #[test]
    fn tfm_dimensions_are_tfm_cs() {
        // characters 65 and 66; 66 has width 2, height 1, depth 2
        let b = tfm_bytes(
            65,
            66,
            &[1 << 24 | 2 << 20 | 1 << 16, 2 << 24 | 1 << 20 | 2 << 16],
            &[0, 500_000, 700_000],
            &[0, 300_000, 800_000],
            &[0, -5, 100_000],
        );
        let t = Tfm::parse(&b).unwrap();
        assert_eq!((t.first, t.last), (65, 66));
        assert_eq!(t.fw_width(65), Some(500_000));
        assert_eq!(t.fw_height(65), Some(800_000));
        assert_eq!(t.fw_depth(65), Some(-5));
        assert_eq!(t.fw_width(66), Some(700_000));
        assert_eq!(t.fw_height(66), Some(300_000));
        assert_eq!(t.fw_depth(66), Some(100_000));
        // above `last`: index 0 (tfm.c's pTeX rule), here an unused slot
        assert_eq!(t.fw_width(200), Some(0));
        // below `first`: tfm.c's fatal "Invalid char"
        assert_eq!(t.fw_width(64), None);
        assert_eq!(t.fw_width(-1), None);
        // a file shorter than it says
        assert!(Tfm::parse(&b[..b.len() - 4]).is_err());
        assert!(Tfm::parse(&b[..20]).is_err());
    }

    fn vf_bytes(body: &[u8]) -> Vec<u8> {
        let mut b = vec![op::PRE, VF_ID, 2, b'h', b'i', 0, 0, 0, 0, 0, 0xA0, 0, 0];
        b.extend_from_slice(body);
        b.extend_from_slice(&[op::POST; 3]);
        b
    }

    fn fnt_def(code: u8, id: &[u8], size: u32, name: &[u8]) -> Vec<u8> {
        let mut b = vec![code];
        b.extend_from_slice(id);
        b.extend_from_slice(&[1, 2, 3, 4]);
        b.extend_from_slice(&size.to_be_bytes());
        b.extend_from_slice(&0x00A0_0000u32.to_be_bytes());
        b.push(0);
        b.push(name.len() as u8);
        b.extend_from_slice(name);
        b
    }

    #[test]
    fn vf_file_fonts_and_packets() {
        let mut body = fnt_def(op::FNT_DEF1, &[200], 0x0010_0000, b"base");
        body.extend(fnt_def(
            op::FNT_DEF4,
            &[0xFF, 0xFF, 0xFF, 0xFE],
            0x0008_0000,
            b"other",
        ));
        // a short packet for `A`: set `B`
        body.extend_from_slice(&[1, b'A', 0, 0, 0, b'B']);
        // a long packet for character 0x41000
        body.push(242);
        body.extend_from_slice(&2u32.to_be_bytes());
        body.extend_from_slice(&0x41000u32.to_be_bytes());
        body.extend_from_slice(&[0; 4]);
        body.extend_from_slice(&[op::PUSH, op::POP]);
        // an empty packet for `C`
        body.extend_from_slice(&[0, b'C', 0, 0, 0]);
        let vf = Vf::parse(&vf_bytes(&body)).unwrap();
        assert_eq!(vf.design_size, 0x00A0_0000);
        assert_eq!(vf.fonts.len(), 2);
        // `fnt_def1`'s number is unsigned, `fnt_def4`'s a signed quad
        assert_eq!(vf.fonts[0].font_id, 200);
        assert_eq!(vf.fonts[0].name, b"base");
        assert_eq!(vf.fonts[0].size, 0x0010_0000);
        assert_eq!(vf.fonts[1].font_id, -2);
        assert_eq!(vf.fonts[1].name, b"other");
        assert_eq!(vf.packet(b'A' as i32).as_deref(), Some(&[b'B'][..]));
        assert_eq!(
            vf.packet(0x41000).as_deref(),
            Some(&[op::PUSH, op::POP][..])
        );
        assert_eq!(vf.packet(b'C' as i32), None);
        assert_eq!(vf.packet(b'D' as i32), None);
        assert_eq!(vf.packet(-1), None);
    }

    #[test]
    fn vf_file_errors_are_xdvipdfmxs() {
        // no POST: the file ends prematurely
        let b = vf_bytes(&[]);
        assert!(Vf::parse(&b[..b.len() - 3]).is_err());
        // a packet longer than the file
        assert_eq!(
            Vf::parse(&vf_bytes(&[200, b'A', 0, 0, 0, 1])).unwrap_err(),
            "VF file ended prematurely."
        );
        // a character of more than 24 bits
        let mut body = vec![242, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0];
        assert!(Vf::parse(&vf_bytes(&body)).is_err());
        // large characters must come in increasing order
        body.clear();
        for ch in [0x50000u32, 0x40000] {
            body.push(242);
            body.extend_from_slice(&0u32.to_be_bytes());
            body.extend_from_slice(&ch.to_be_bytes());
            body.extend_from_slice(&[0; 4]);
        }
        assert_eq!(
            Vf::parse(&vf_bytes(&body)).unwrap_err(),
            "Unexpected character code in vf file"
        );
        // a negative font size
        let body = fnt_def(op::FNT_DEF1, &[0], 0x8000_0000, b"x");
        assert!(Vf::parse(&vf_bytes(&body)).is_err());
        // an unknown code quits quietly with what was read
        let mut body = vec![1, b'A', 0, 0, 0, b'B'];
        body.push(250);
        body.extend_from_slice(&[1, b'Z', 0, 0, 0, b'B']);
        let vf = Vf::parse(&vf_bytes(&body)).unwrap();
        assert!(vf.packet(b'A' as i32).is_some());
        assert!(vf.packet(b'Z' as i32).is_none());
        // without the signature ("VF file may be corrupt") the file is
        // read on from its second byte: here a packet for `A`
        let vf = Vf::parse(&[0, 1, b'A', 0, 0, 0, b'B', op::POST]).unwrap();
        assert_eq!(vf.packet(b'A' as i32).as_deref(), Some(&[b'B'][..]));
        assert!(Vf::parse(&[op::POST]).is_err());
    }

    fn cmds(b: &[u8]) -> Vec<Result<Cmd<'_>, String>> {
        let mut p = Packet::new(b);
        let mut v = Vec::new();
        while let Some(c) = p.next() {
            let stop = c.is_err();
            v.push(c);
            if stop {
                break;
            }
        }
        v
    }

    #[test]
    fn packet_commands_read_numbers_as_vf_c_does() {
        use op::*;
        #[rustfmt::skip]
        let b = [
            65, // set_char_65
            SET1, 200, // unsigned
            SET1 + 1, 0x12, 0x34, // set2
            SET3, 0xFF, 0x00, 0x01, // set3: unsigned
            PUT1, 7,
            SET_RULE, 0, 0, 0, 5, 0xFF, 0xFF, 0xFF, 0xFE,
            PUT_RULE, 0, 0, 0, 1, 0, 0, 0, 2,
            NOP, PUSH, POP,
            RIGHT1, 0x80, // signed
            RIGHT1 + 2, 0x80, 0, 0,
            RIGHT4, 0, 1, 0, 0,
            W0, W1, 3, W4, 0xFF, 0xFF, 0xFF, 0xFF,
            X0, X1, 4,
            DOWN1, 0xFF,
            Y0, Y1, 5,
            Z0, Z1, 6,
            FNT_NUM_0 + 3,
            FNT1, 200, // signed: -56
            FNT4, 0, 0, 1, 0,
            XXX1, 2, b'h', b'i',
            PTEXDIR, 1,
        ];
        let got: Vec<Cmd> = cmds(&b).into_iter().map(Result::unwrap).collect();
        assert_eq!(
            got,
            vec![
                Cmd::Set(65),
                Cmd::Set(200),
                Cmd::Set(0x1234),
                Cmd::Set(0xFF0001),
                Cmd::Put(7),
                Cmd::SetRule {
                    height: 5,
                    width: -2
                },
                Cmd::PutRule {
                    height: 1,
                    width: 2
                },
                Cmd::Nop,
                Cmd::Push,
                Cmd::Pop,
                Cmd::Right(-128),
                Cmd::Right(-0x80_0000),
                Cmd::Right(0x1_0000),
                Cmd::W0,
                Cmd::W(3),
                Cmd::W(-1),
                Cmd::X0,
                Cmd::X(4),
                Cmd::Down(-1),
                Cmd::Y0,
                Cmd::Y(5),
                Cmd::Z0,
                Cmd::Z(6),
                Cmd::Fnt(3),
                Cmd::Fnt(-56),
                Cmd::Fnt(256),
                Cmd::Special(b"hi"),
                Cmd::Dir(1),
            ]
        );
    }

    #[test]
    fn packet_errors_are_vf_cs() {
        use op::*;
        // a number cut short
        assert_eq!(cmds(&[RIGHT4, 0, 0, 0])[0], Err(PREMATURE.into()));
        assert_eq!(cmds(&[SET1])[0], Err(PREMATURE.into()));
        assert_eq!(
            cmds(&[SET_RULE, 0, 0, 0, 1, 0, 0])[0],
            Err(PREMATURE.into())
        );
        // a special longer than the packet
        assert!(cmds(&[XXX1, 3, b'a'])[0].is_err());
        // four-byte characters
        assert!(cmds(&[SET4, 0, 0, 0, 1])[0].is_err());
        assert!(cmds(&[PUT4, 0, 0, 0, 1])[0].is_err());
        // opcodes a packet may not hold: bop, eop, fnt_def, pre, post
        for o in [139u8, 140, FNT_DEF1, PRE, POST, 249, 250, 254] {
            assert!(cmds(&[o])[0].is_err(), "{o}");
        }
        // a special whose four-byte length is negative: skipped over
        // nothing, the bytes after it are read as commands
        assert_eq!(
            cmds(&[XXX4, 0xFF, 0xFF, 0xFF, 0xFF, 65]),
            vec![Ok(Cmd::BadSpecial(-1)), Ok(Cmd::Set(65))]
        );
        // the commands before an error are run
        let c = cmds(&[65, PRE, 66]);
        assert_eq!(c.len(), 2);
        assert_eq!(c[0], Ok(Cmd::Set(65)));
        assert!(c[1].is_err());
    }

    #[test]
    fn warning_specials() {
        assert!(is_warning_special(b"Warning: missing glyph"));
        assert!(is_warning_special(b"   Warning:x"));
        assert!(!is_warning_special(b"warning: x"));
        assert!(!is_warning_special(b"\tWarning: x"));
        assert!(!is_warning_special(b"pdf:literal"));
        assert!(!is_warning_special(b""));
    }
}
