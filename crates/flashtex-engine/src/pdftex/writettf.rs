//! `writettf.c`, ported: TrueType font embedding and subsetting
//! (`writettf`), and OpenType (CFF) embedding (`writeotf`).
//!
//! A subsetted TrueType font is rebuilt table by table into the font
//! buffer: the glyphs TeX used (by glyph name through the font's `post`
//! table, by `uniXXXX` through its Unicode `cmap`, by `indexNNN`, or, for a
//! subfont, by character code through the map entry's `cmap`), renumbered
//! in the order they are met, with new `cmap`, `name` (subset tags),
//! `OS/2`, `post`, `loca`, `hmtx`, `head` tables and checksums. A whole
//! TrueType font is copied table by table; an OpenType font's `CFF ` table
//! is copied as it is.
//!
//! The C file keeps its state in file-level statics; here they are the
//! fields of [`Ttf`], alive for one call, except the `cmap` tables read so
//! far (`ttf_cmap_tree`), which pdfTeX keeps for the whole run
//! (`writefont::State::ttf_cmaps`).

use super::cfile::{CFile, Whence};
use super::fonts::{Fonts, NOTDEF};
use super::macnames::{AMBIGUOUS_NAMES, MAC_GLYPH_NAMES};
use super::output::set_cur_file_name;
use super::writefont::{
    FdEntry, ASCENT_CODE, CAPHEIGHT_CODE, DESCENT_CODE, FONTBBOX1_CODE, FONTBBOX2_CODE,
    FONTBBOX3_CODE, FONTBBOX4_CODE, FONTNAME_CODE, ITALIC_ANGLE_CODE, XHEIGHT_CODE,
};
use crate::generated::Globals;
use crate::resolver::Format;

const DEFAULT_NTABS: u16 = 14;
const NEW_CMAP_SIZE: usize = 2;
const NMACGLYPHS: usize = 258;
const TABDIR_OFF: usize = 12;

const ARG_1_AND_2_ARE_WORDS: u16 = 1 << 0;
const WE_HAVE_A_SCALE: u16 = 1 << 3;
const MORE_COMPONENTS: u16 = 1 << 5;
const WE_HAVE_AN_X_AND_Y_SCALE: u16 = 1 << 6;
const WE_HAVE_A_TWO_BY_TWO: u16 = 1 << 7;
const WE_HAVE_INSTRUCTIONS: u16 = 1 << 8;

/// `BYTE_ENCODING_LENGTH`, `TRIMMED_TABLE_MAP_LENGTH`,
/// `SEG_MAP_DELTA_LENGTH`, `CMAP_ENTRY_LENGTH`.
const BYTE_ENCODING_LENGTH: i64 = 256 + 3 * 2;
const TRIMMED_TABLE_MAP_LENGTH: i64 = 2 * (5 + 256);
const SEG_MAP_DELTA_LENGTH: i64 = (16 + 256) * 2;
const CMAP_ENTRY_LENGTH: i64 = 2 * 2 + 4;

/// `newtabnames`: the tables of a subsetted font, in directory order.
const NEWTABNAMES: [&[u8; 4]; 14] = [
    b"OS/2", b"PCLT", b"cmap", b"cvt ", b"fpgm", b"glyf", b"head", b"hhea", b"hmtx", b"loca",
    b"maxp", b"name", b"post", b"prep",
];

/// `dirtab_entry`.
#[derive(Clone, Copy, Default)]
struct DirTab {
    tag: [u8; 4],
    checksum: u32,
    offset: u32,
    length: u32,
}

/// `glyph_entry`.
#[derive(Clone)]
struct Glyph {
    offset: i32,
    newoffset: i32,
    adv_width: u16,
    lsb: i16,
    /// `name` (`notdef` until the `post` table names it).
    name: Vec<u8>,
    newindex: i16,
    name_index: u16,
}

impl Default for Glyph {
    fn default() -> Glyph {
        Glyph {
            offset: 0,
            newoffset: 0,
            adv_width: 0,
            lsb: 0,
            name: NOTDEF.to_vec(),
            newindex: -1,
            name_index: 0,
        }
    }
}

/// `ttfenc_entry`.
#[derive(Clone, Default)]
struct TtfEnc {
    name: Vec<u8>,
    code: i64,
    newindex: i64,
}

/// `name_record`.
#[derive(Clone, Copy, Default)]
struct NameRecord {
    platform_id: u16,
    encoding_id: u16,
    language_id: u16,
    name_id: u16,
    length: u16,
    offset: u16,
    new_offset: u16,
    new_length: u16,
}

/// `cmap_entry` (the new `cmap` subtables).
#[derive(Clone, Copy, Default)]
struct CmapEntry {
    platform_id: u16,
    encoding_id: u16,
    offset: u32,
    format: u16,
}

/// The state of one `writettf`/`writeotf` call.
struct Ttf<'a> {
    g: &'a mut Globals,
    file: CFile,
    /// The font buffer (`fb_array`) and its write position (`fb_ptr`).
    fb: Vec<u8>,
    fb_pos: usize,
    ntabs: u16,
    upem: u16,
    post_format: u32,
    loca_format: i16,
    last_glyf_offset: u32,
    glyphs_count: u16,
    new_glyphs_count: u16,
    nhmtxs: u16,
    new_ntabs: u16,
    glyph_tab: Vec<Glyph>,
    /// `glyph_tab == NULL` (writeotf reads `post` without glyphs).
    no_glyphs: bool,
    glyph_index: Vec<i64>,
    name_tab: Vec<NameRecord>,
    name_buf: Vec<u8>,
    dir_tab: Vec<DirTab>,
    checksum: u32,
    tab_length: u32,
    tmp_ulong: u32,
    check_sum_adjustment_offset: u32,
    ttfenc_tab: Vec<TtfEnc>,
    fd: &'a mut FdEntry,
    /// The map entry's font file name (`fd_cur->fm->ff_name`), the key of
    /// the `cmap` cache.
    ff_name: Vec<u8>,
    subsetted: bool,
    /// `fd_cur->fm->subfont->charcodes`, for a subfont.
    subfont: Option<Vec<i32>>,
    pid: i32,
    eid: i32,
    tfm_name: Vec<u8>,
    /// `fd_cur->fe->glyph_names`.
    enc_glyph_names: Option<Vec<Vec<u8>>>,
}

impl Ttf<'_> {
    // --- the font buffer (utils.c's fb_* over fb_array) ---

    /// `ttf_putchar` (`fb_putchar`).
    fn putchar(&mut self, b: u8) {
        if self.fb_pos < self.fb.len() {
            self.fb[self.fb_pos] = b;
        } else {
            self.fb.resize(self.fb_pos, 0);
            self.fb.push(b);
        }
        self.fb_pos += 1;
    }

    /// `ttf_offset` (`fb_offset`).
    fn offset(&self) -> u32 {
        self.fb_pos as u32
    }

    /// `ttf_seek_outbuf` (`fb_seek`).
    fn seek_outbuf(&mut self, offset: u32) {
        self.fb_pos = offset as usize;
    }

    // --- reading ---

    /// `ttf_getnum`: an `s`-byte big-endian number (`long`).
    fn getnum(&mut self, s: usize) -> i64 {
        let mut i: i64 = 0;
        for _ in 0..s {
            let c = self.file.getc();
            if c < 0 {
                self.g.pdftex_fail("unexpected EOF");
            }
            i = (i << 8) + c as i64;
        }
        i
    }

    fn get_byte(&mut self) -> u8 {
        self.getnum(1) as u8
    }
    fn get_char(&mut self) -> i8 {
        self.getnum(1) as i8
    }
    fn get_ushort(&mut self) -> u16 {
        self.getnum(2) as u16
    }
    fn get_short(&mut self) -> i16 {
        self.getnum(2) as i16
    }
    fn get_ulong(&mut self) -> u32 {
        self.getnum(4) as u32
    }
    fn get_fword(&mut self) -> i16 {
        self.getnum(2) as i16
    }
    fn get_ufword(&mut self) -> u16 {
        self.getnum(2) as u16
    }
    fn skip(&mut self, n: usize) {
        self.getnum(n);
    }

    // --- writing, with the running checksum ---

    /// `ttf_addchksm`.
    fn addchksm(&mut self, b: u8) -> u8 {
        self.tmp_ulong = (self.tmp_ulong << 8).wrapping_add(b as u32);
        self.tab_length = self.tab_length.wrapping_add(1);
        if self.tab_length % 4 == 0 {
            self.checksum = self.checksum.wrapping_add(self.tmp_ulong);
            self.tmp_ulong = 0;
        }
        b
    }

    /// `ttf_getchksm`: pad the table to a multiple of 4 bytes.
    fn getchksm(&mut self) -> u32 {
        while self.tab_length % 4 != 0 {
            let b = self.addchksm(0);
            self.putchar(b);
        }
        self.checksum
    }

    /// `ttf_putnum`: `n` as an `s`-byte big-endian number; returns `n`.
    fn putnum(&mut self, s: usize, n: i64) -> i64 {
        let mut i = n;
        let mut buf = [0u8; 4];
        for b in buf.iter_mut().take(s) {
            *b = (i & 0xFF) as u8;
            i >>= 8;
        }
        for k in (0..s).rev() {
            let b = self.addchksm(buf[k]);
            self.putchar(b);
        }
        n
    }

    fn put_byte(&mut self, n: i64) {
        self.putnum(1, n);
    }
    fn put_char(&mut self, n: i64) {
        self.putnum(1, n);
    }
    fn put_ushort(&mut self, n: i64) -> u16 {
        self.putnum(2, n) as u16
    }
    fn put_short(&mut self, n: i64) -> i16 {
        self.putnum(2, n) as i16
    }
    fn put_ulong(&mut self, n: i64) {
        self.putnum(4, n);
    }
    fn put_fixed(&mut self, n: i64) {
        self.putnum(4, n);
    }
    fn put_ufword(&mut self, n: i64) {
        self.putnum(2, n);
    }

    /// `copy_byte`.
    fn copy_byte(&mut self) {
        let b = self.get_byte();
        self.put_byte(b as i64);
    }
    /// `copy_char`.
    fn copy_char(&mut self) {
        let c = self.get_char();
        self.put_char(c as i64);
    }
    /// `copy_short`.
    fn copy_short(&mut self) -> i16 {
        let s = self.get_short();
        self.put_short(s as i64)
    }
    /// `copy_ushort`.
    fn copy_ushort(&mut self) -> u16 {
        let s = self.get_ushort();
        self.put_ushort(s as i64)
    }

    /// `ttf_funit`: font units to thousandths of the em.
    fn funit(&self, n: i64) -> i64 {
        let upem = self.upem as i64;
        if upem == 0 {
            return 0; // C divides by zero
        }
        if n < 0 {
            -((-n / upem) * 1000 + ((-n % upem) * 1000) / upem)
        } else {
            (n / upem) * 1000 + ((n % upem) * 1000) / upem
        }
    }

    /// `ttf_ncopy`.
    fn ncopy(&mut self, n: i64) {
        let mut n = n;
        while n > 0 {
            self.copy_byte();
            n -= 1;
        }
    }

    /// `ttf_name_lookup`: the directory entry of table `s`.
    fn name_lookup(&mut self, s: &[u8], required: bool) -> Option<usize> {
        let found = self.dir_tab.iter().position(|t| &t.tag[..] == s);
        if found.is_none() && required {
            self.g.pdftex_fail(&format!(
                "can't find table `{}'",
                String::from_utf8_lossy(s)
            ));
        }
        found
    }

    /// `ttf_seek_tab`.
    fn seek_tab(&mut self, name: &[u8], offset: i64) -> usize {
        let tab = self.name_lookup(name, true).unwrap();
        let o = self.dir_tab[tab].offset as i64 + offset;
        self.seek_off(o);
        tab
    }

    /// `ttf_seek_off` (`xfseek(INFILE, offset, SEEK_SET, cur_file_name)`).
    fn seek_off(&mut self, offset: i64) {
        if !self.file.seek(offset, Whence::Set) {
            self.xfseek_fail();
        }
    }

    fn xfseek_fail(&mut self) -> ! {
        let name = super::output::cur_file_name().unwrap_or_default();
        let prog = crate::system::invocation_name();
        eprintln!(
            "{prog}: fseek({}, ...) failed: Invalid argument",
            String::from_utf8_lossy(&name)
        );
        crate::system::exit_process(self.g, 1)
    }

    /// `ttf_copy_encoding`: the characters TeX used, with their glyph
    /// names (a reencoded font) or character codes (a subfont).
    fn copy_encoding(&mut self) {
        if let Some(glyph_names) = self.enc_glyph_names.clone() {
            for e in self.ttfenc_tab.iter_mut() {
                e.name = NOTDEF.to_vec();
            }
            // a workaround for a bug of AcroReader 4.0
            if glyph_names[97].as_slice() == b"a" {
                self.fd.tx_tree.get_or_insert_with(Default::default).insert(b'a' as i32);
            }
            // take over collected characters from TeX, reencode them
            let tx: Vec<i32> = self.fd.tx_tree.iter().flatten().copied().collect();
            for q in tx {
                self.ttfenc_tab[q as usize].name = glyph_names[q as usize].clone();
            }
            self.make_subset_tag();
        } else if let Some(charcodes) = self.subfont.clone() {
            for e in self.ttfenc_tab.iter_mut() {
                e.code = -1;
            }
            // take over collected characters from TeX
            let tx: Vec<i32> = self.fd.tx_tree.iter().flatten().copied().collect();
            for q in tx {
                let code = charcodes[q as usize] as i64;
                self.ttfenc_tab[q as usize].code = code;
                if code == -1 {
                    let tfm = String::from_utf8_lossy(&self.tfm_name).into_owned();
                    self.g.pdftex_warn(&format!(
                        "character {q} in subfont {tfm} is not mapped to any charcode"
                    ));
                } else {
                    let buf = format!("/c{:04X}", code as i32);
                    self.fd
                        .gl_tree
                        .get_or_insert_with(Default::default)
                        .insert(buf.into_bytes());
                }
            }
            self.make_subset_tag();
        }
    }

    /// `make_subset_tag(fd_cur)`.
    fn make_subset_tag(&mut self) {
        let empty = Default::default();
        let glyphs = self.fd.gl_tree.as_ref().unwrap_or(&empty);
        let name = self.fd.fontname.clone().unwrap_or_default();
        let tag = self.g.make_subset_tag(glyphs.iter(), &name);
        self.fd.subset_tag = Some(tag);
    }

    /// `ttf_read_name`: the name table, and the PostScript name in it.
    fn read_name(&mut self) {
        let tab = self.seek_tab(b"name", 2);
        let n = self.get_ushort() as usize;
        self.name_tab = vec![NameRecord::default(); n];
        let size = self.dir_tab[tab].length as i64 - (3 * 2 + n as i64 * 6 * 2);
        self.skip(2);
        for i in 0..n {
            self.name_tab[i].platform_id = self.get_ushort();
            self.name_tab[i].encoding_id = self.get_ushort();
            self.name_tab[i].language_id = self.get_ushort();
            self.name_tab[i].name_id = self.get_ushort();
            self.name_tab[i].length = self.get_ushort();
            self.name_tab[i].offset = self.get_ushort();
        }
        let size = size.max(0) as usize;
        self.name_buf = Vec::with_capacity(size);
        for _ in 0..size {
            let c = self.get_char();
            self.name_buf.push(c as u8);
        }
        // look for PS font name
        for i in 0..n {
            let r = self.name_tab[i];
            if r.platform_id == 1 && r.encoding_id == 0 && r.name_id == 6 {
                let s = self.name_bytes(r.offset as usize, r.length as usize);
                self.fd.fontname = Some(strip_spaces_and_delims(&s));
                self.fd.font_dim[FONTNAME_CODE].set = true;
                break;
            }
        }
        if !self.fd.font_dim[FONTNAME_CODE].set {
            for i in 0..n {
                let r = self.name_tab[i];
                if r.platform_id == 3
                    && (r.encoding_id == 0 || r.encoding_id == 1)
                    && r.name_id == 6
                {
                    let mut buf = Vec::new();
                    let mut j = 0;
                    while j < r.length as usize {
                        let b = self
                            .name_buf
                            .get(r.offset as usize + j + 1)
                            .copied()
                            .unwrap_or(0);
                        buf.push(b);
                        j += 2;
                    }
                    // strlen(buf): the name ends at a NUL
                    if let Some(z) = buf.iter().position(|&b| b == 0) {
                        buf.truncate(z);
                    }
                    self.fd.fontname = Some(strip_spaces_and_delims(&buf));
                    self.fd.font_dim[FONTNAME_CODE].set = true;
                    break;
                }
            }
        }
    }

    /// `name_buf + offset`, `length` bytes (C reads past the buffer's end
    /// for a bad record; here that is zeros).
    fn name_bytes(&self, offset: usize, length: usize) -> Vec<u8> {
        (0..length)
            .map(|k| self.name_buf.get(offset + k).copied().unwrap_or(0))
            .collect()
    }

    /// `ttf_read_mapx` (the `maxp` table).
    fn read_mapx(&mut self) {
        self.seek_tab(b"maxp", 4);
        self.glyphs_count = self.get_ushort();
        self.glyph_tab = vec![Glyph::default(); 1 + self.glyphs_count as usize];
        self.no_glyphs = false;
        self.glyph_index = vec![0; self.glyphs_count as usize + 1];
        self.glyph_index[0] = 0; // index of ".notdef" glyph
        if self.glyph_index.len() > 1 {
            self.glyph_index[1] = 1; // index of ".null" glyph
        }
    }

    /// `ttf_read_head`.
    fn read_head(&mut self) {
        self.seek_tab(b"head", 2 * 4 + 2 * 4 + 2);
        self.upem = self.get_ushort();
        self.skip(16);
        for code in [FONTBBOX1_CODE, FONTBBOX2_CODE, FONTBBOX3_CODE, FONTBBOX4_CODE] {
            let v = self.get_fword() as i64;
            self.fd.font_dim[code].val = self.funit(v) as i32;
        }
        for code in [FONTBBOX1_CODE, FONTBBOX2_CODE, FONTBBOX3_CODE, FONTBBOX4_CODE] {
            self.fd.font_dim[code].set = true;
        }
        self.skip(2 * 2 + 2);
        self.loca_format = self.get_short();
    }

    /// `ttf_read_hhea`.
    fn read_hhea(&mut self) {
        self.seek_tab(b"hhea", 4);
        let a = self.get_fword() as i64;
        self.fd.font_dim[ASCENT_CODE].val = self.funit(a) as i32;
        let d = self.get_fword() as i64;
        self.fd.font_dim[DESCENT_CODE].val = self.funit(d) as i32;
        self.fd.font_dim[ASCENT_CODE].set = true;
        self.fd.font_dim[DESCENT_CODE].set = true;
        self.skip(2 + 2 + 3 * 2 + 8 * 2);
        self.nhmtxs = self.get_ushort();
    }

    /// `ttf_read_pclt`.
    fn read_pclt(&mut self) {
        if self.name_lookup(b"PCLT", false).is_none() {
            return;
        }
        self.seek_tab(b"PCLT", 4 + 4 + 2);
        let x = self.get_ushort() as i64;
        self.fd.font_dim[XHEIGHT_CODE].val = self.funit(x) as i32;
        self.skip(2 * 2);
        let c = self.get_ushort() as i64;
        self.fd.font_dim[CAPHEIGHT_CODE].val = self.funit(c) as i32;
        self.fd.font_dim[XHEIGHT_CODE].set = true;
        self.fd.font_dim[CAPHEIGHT_CODE].set = true;
    }

    /// `ttf_read_hmtx`.
    fn read_hmtx(&mut self) {
        self.seek_tab(b"hmtx", 0);
        let n = self.nhmtxs as usize;
        let count = self.glyphs_count as usize;
        let mut g = 0;
        while g < n {
            let w = self.get_ufword();
            let l = self.get_ufword() as i16;
            if let Some(e) = self.glyph_tab.get_mut(g) {
                e.adv_width = w;
                e.lsb = l;
            }
            g += 1;
        }
        if n < count {
            let last = if g > 0 {
                self.glyph_tab.get(g - 1).map_or(0, |e| e.adv_width)
            } else {
                0
            };
            while g < count {
                let l = self.get_ufword() as i16;
                self.glyph_tab[g].adv_width = last;
                self.glyph_tab[g].lsb = l;
                g += 1;
            }
        }
    }

    /// `ttf_read_post`: the italic angle, and the glyph names.
    fn read_post(&mut self) {
        let tab = self.seek_tab(b"post", 0);
        self.post_format = self.get_ulong();
        let italic_angle = self.get_ulong();
        let mut int_part = (italic_angle >> 16) as i64;
        let mut sign = 1i64;
        if int_part > 0x7FFF {
            // a negative number
            int_part = 0x10000 - int_part;
            sign = -1;
        }
        let frac_part = (italic_angle % 0x10000) as i64;
        self.fd.font_dim[ITALIC_ANGLE_CODE].val =
            (sign as f64 * (int_part as f64 + frac_part as f64 * 1.0 / 65536.0)) as i32;
        self.fd.font_dim[ITALIC_ANGLE_CODE].set = true;
        if self.no_glyphs {
            return; // being called from writeotf()
        }
        self.skip(2 * 2 + 5 * 4);
        let count = self.glyph_tab.len();
        match self.post_format {
            0x10000 => {
                for (i, n) in MAC_GLYPH_NAMES.iter().enumerate().take(count) {
                    self.glyph_tab[i].name = n.to_vec();
                    self.glyph_tab[i].name_index = i as u16;
                }
            }
            0x20000 => {
                let nnames = self.get_ushort() as usize;
                for i in 0..nnames {
                    let v = self.get_ushort();
                    if let Some(e) = self.glyph_tab.get_mut(i) {
                        e.name_index = v;
                    }
                }
                let length = self.dir_tab[tab].length as i64
                    - (self.file.tell() - self.dir_tab[tab].offset as i64);
                // glyph_name_buf: the table's Pascal strings, each ended
                // by a NUL
                let mut buf: Vec<u8> = Vec::new();
                while (buf.len() as i64) < length {
                    let k = self.get_byte();
                    for _ in 0..k {
                        let c = self.get_char() as u8;
                        buf.push(c);
                    }
                    buf.push(0);
                }
                for i in 0..nnames.min(count) {
                    let ni = self.glyph_tab[i].name_index as usize;
                    self.glyph_tab[i].name = if ni < NMACGLYPHS {
                        MAC_GLYPH_NAMES[ni].to_vec()
                    } else {
                        // p = strend(p) + 1, k times (C walks past the
                        // buffer for a bad index; here that gives "")
                        let mut p = 0usize;
                        for _ in 0..ni - NMACGLYPHS {
                            p = buf[p.min(buf.len())..]
                                .iter()
                                .position(|&b| b == 0)
                                .map_or(buf.len(), |z| p + z + 1);
                        }
                        let rest = &buf[p.min(buf.len())..];
                        rest[..rest.iter().position(|&b| b == 0).unwrap_or(rest.len())].to_vec()
                    };
                }
            }
            0x30000 => self.post_format_3(),
            f => {
                self.g.pdftex_warn(&format!(
                    "unsupported format ({f:08X}) of `post' table, assuming 3.0"
                ));
                self.post_format_3();
            }
        }
    }

    fn post_format_3(&mut self) {
        let count = self.glyph_tab.len();
        for i in 0..NMACGLYPHS.min(count) {
            self.glyph_tab[i].name_index = i as u16;
        }
    }

    /// `ttf_read_loca`.
    fn read_loca(&mut self) {
        self.seek_tab(b"loca", 0);
        let n = self.glyphs_count as usize + 1;
        for i in 0..n {
            self.glyph_tab[i].offset = if self.loca_format != 0 {
                self.get_ulong() as i32
            } else {
                (self.get_ushort() as i32) << 1
            };
        }
    }

    /// `ttf_read_tabdir`.
    fn read_tabdir(&mut self) {
        self.skip(4); // ignore the sfnt number
        self.ntabs = self.get_ushort();
        self.dir_tab = vec![DirTab::default(); self.ntabs as usize];
        self.skip(3 * 2);
        for t in 0..self.ntabs as usize {
            for i in 0..4 {
                self.dir_tab[t].tag[i] = self.get_char() as u8;
            }
            self.dir_tab[t].checksum = self.get_ulong();
            self.dir_tab[t].offset = self.get_ulong();
            self.dir_tab[t].length = self.get_ulong();
        }
    }

    /// `ttf_read_cmap`: the format-4 `cmap` subtable for (pid, eid) as a
    /// table from character code to glyph index (-1 for none), read once
    /// per font file and run.
    fn read_cmap(&mut self, st: &mut Fonts, pid: i32, eid: i32, warn: bool) -> Option<Vec<i32>> {
        let key = (self.ff_name.clone(), pid, eid);
        if let Some(t) = st.wf.ttf_cmaps.get(&key) {
            return Some(t.clone());
        }
        self.seek_tab(b"cmap", 2); // skip the table version number (=0)
        let ncmapsubtabs = self.get_ushort();
        let cmap_offset = self.file.tell() - 2 * 2;
        let mut found = false;
        for _ in 0..ncmapsubtabs {
            let tmp_pid = self.get_ushort() as i32;
            let tmp_eid = self.get_ushort() as i32;
            let tmp_offset = self.get_ulong() as i64;
            if tmp_pid == pid && tmp_eid == eid {
                self.seek_off(cmap_offset + tmp_offset);
                let format = self.get_ushort();
                if format == 4 {
                    found = true;
                    break;
                }
                if warn {
                    self.g
                        .pdftex_warn(&format!("cmap format {format} unsupported"));
                }
                return None;
            }
        }
        if !found {
            if warn {
                self.g.pdftex_warn(&format!(
                    "cannot find cmap subtable for (pid,eid) = ({pid}, {eid})"
                ));
            }
            return None;
        }
        // read_cmap_format_4:
        let mut table = vec![-1i32; 0x10000];
        let mut length = self.get_ushort() as i64; // length of subtable
        self.get_ushort(); // skip the version number
        let seg_count = (self.get_ushort() / 2) as usize;
        self.get_ushort(); // skip searchRange
        self.get_ushort(); // skip entrySelector
        self.get_ushort(); // skip rangeShift
        let mut end = vec![0u16; seg_count];
        let mut start = vec![0u16; seg_count];
        let mut delta = vec![0u16; seg_count];
        let mut range = vec![0u16; seg_count];
        for e in end.iter_mut() {
            *e = self.get_ushort();
        }
        self.get_ushort(); // skip reversedPad
        for e in start.iter_mut() {
            *e = self.get_ushort();
        }
        for e in delta.iter_mut() {
            *e = self.get_ushort();
        }
        for e in range.iter_mut() {
            *e = self.get_ushort();
        }
        length -= 8 * 2 + 4 * seg_count as i64 * 2;
        let n = (length / 2).max(0) as usize; // number of glyphID's
        let mut glyph_id = vec![0u16; n];
        for e in glyph_id.iter_mut() {
            *e = self.get_ushort();
        }
        for s in 0..seg_count {
            let mut i = start[s] as i64;
            while i <= end[s] as i64 {
                if i == 0xFFFF {
                    break;
                }
                if range[s] != 0xFFFF {
                    let index: i64 = if range[s] == 0 {
                        (delta[s] as i64 + i) & 0xFFFF
                    } else {
                        let k = (i - start[s] as i64) + range[s] as i64 / 2 + s as i64
                            - seg_count as i64;
                        // (C asserts 0 <= k < n)
                        let mut index = if k >= 0 && (k as usize) < n {
                            glyph_id[k as usize] as i64
                        } else {
                            0
                        };
                        if index != 0 {
                            index = (index + delta[s] as i64) & 0xFFFF;
                        }
                        index
                    };
                    if index >= self.glyphs_count as i64 {
                        self.g.pdftex_fail(&format!(
                            "cmap: glyph index {index} out of range [0..{})",
                            self.glyphs_count
                        ));
                    }
                    if table[i as usize] != -1 {
                        self.g.pdftex_warn(&format!(
                            "cmap: multiple glyphs are mapped to unicode {i:04X}, \
                             only {} will be used (glyph {index} being ignored)",
                            table[i as usize]
                        ));
                    } else {
                        table[i as usize] = index as i32;
                    }
                }
                i += 1;
            }
        }
        st.wf.ttf_cmaps.insert(key, table.clone());
        Some(table)
    }

    /// `ttf_read_font`.
    fn read_font(&mut self) {
        self.read_tabdir();
        for t in [b"PCLT", b"fpgm", b"cvt ", b"prep"] {
            if self.name_lookup(t, false).is_none() {
                self.new_ntabs = self.new_ntabs.wrapping_sub(1);
            }
        }
        self.read_mapx();
        self.read_head();
        self.read_hhea();
        self.read_pclt();
        self.read_hmtx();
        self.read_post();
        self.read_loca();
        self.read_name();
    }

    /// `ttf_reset_chksm`.
    fn reset_chksm(&mut self, tab: usize) {
        self.checksum = 0;
        self.tab_length = 0;
        self.tmp_ulong = 0;
        self.dir_tab[tab].offset = self.offset();
        if self.dir_tab[tab].offset % 4 != 0 {
            let tag = String::from_utf8_lossy(&self.dir_tab[tab].tag).into_owned();
            self.g
                .pdftex_warn(&format!("offset of `{tag}' is not a multiple of 4"));
        }
    }

    /// `ttf_set_chksm`.
    fn set_chksm(&mut self, tab: usize) {
        self.dir_tab[tab].length = self.offset().wrapping_sub(self.dir_tab[tab].offset);
        self.dir_tab[tab].checksum = self.getchksm();
    }

    /// `ttf_copytab`.
    fn copytab(&mut self, name: &[u8]) {
        let tab = self.seek_tab(name, 0);
        self.reset_chksm(tab);
        for _ in 0..self.dir_tab[tab].length {
            self.copy_char();
        }
        self.set_chksm(tab);
    }

    /// `ttf_byte_encoding`.
    fn byte_encoding(&mut self) {
        self.put_ushort(0); // format number (0: byte encoding table)
        self.put_ushort(BYTE_ENCODING_LENGTH); // length of table
        self.put_ushort(0); // version number
        for i in 0..256 {
            let e = self.ttfenc_tab[i].clone();
            if e.newindex < 256 {
                self.put_byte(e.newindex);
            } else {
                if e.name.as_slice() != NOTDEF {
                    let mut msg = b"glyph `".to_vec();
                    msg.extend_from_slice(&e.name);
                    msg.extend_from_slice(b"' has been mapped to `.notdef' in `ttf_byte_encoding' cmap table");
                    self.g.pdftex_warn_bytes(&msg);
                }
                self.put_byte(0); // notdef
            }
        }
    }

    /// `ttf_trimmed_table_map`.
    fn trimmed_table_map(&mut self) {
        self.put_ushort(6); // format number (6): trimmed table mapping
        self.put_ushort(TRIMMED_TABLE_MAP_LENGTH);
        self.put_ushort(0); // version number (0)
        self.put_ushort(0); // first character code
        self.put_ushort(256); // number of character code in table
        for i in 0..256 {
            let v = self.ttfenc_tab[i].newindex;
            self.put_ushort(v);
        }
    }

    /// `ttf_seg_map_delta`.
    fn seg_map_delta(&mut self) {
        for v in [
            4, // format number (4: segment mapping to delta values)
            SEG_MAP_DELTA_LENGTH,
            0,      // version number
            4,      // 2*segCount
            4,      // searchRange
            1,      // entrySelector
            0,      // rangeShift
            0xF0FF, // endCount[0]
            0xFFFF, // endCount[1]
            0,      // reversedPad
            0xF000, // startCount[0]
            0xFFFF, // startCount[1]
            0,      // idDelta[0]
            1,      // idDelta[1]
            2 * 2,  // idRangeOffset[0]
            0,      // idRangeOffset[1]
        ] {
            self.put_ushort(v);
        }
        for i in 0..256 {
            let v = self.ttfenc_tab[i].newindex;
            self.put_ushort(v);
        }
    }

    /// `ttf_select_cmap`.
    fn select_cmap(&self) -> [CmapEntry; NEW_CMAP_SIZE] {
        [
            CmapEntry {
                platform_id: 1, // Macintosh
                encoding_id: 0, // Symbol; ignore code page
                offset: 0,
                format: if self.new_glyphs_count < 256 { 0 } else { 6 },
            },
            CmapEntry {
                platform_id: 3, // Microsoft
                encoding_id: 0, // Symbol; ignore code page
                offset: 0,
                format: 4, // segment mapping to delta
            },
        ]
    }

    /// `ttf_write_cmap`.
    fn write_cmap(&mut self) {
        let tab = self.name_lookup(b"cmap", true).unwrap();
        let mut cmaps = self.select_cmap();
        self.reset_chksm(tab);
        self.put_ushort(0); // table version number (0)
        self.put_ushort(NEW_CMAP_SIZE as i64); // number of encoding tables
        let mut offset = 2 * 2 + NEW_CMAP_SIZE as i64 * CMAP_ENTRY_LENGTH;
        for ce in cmaps.iter_mut() {
            ce.offset = offset as u32;
            match ce.format {
                0 => offset += BYTE_ENCODING_LENGTH,
                4 => offset += SEG_MAP_DELTA_LENGTH,
                6 => offset += TRIMMED_TABLE_MAP_LENGTH,
                _ => self
                    .g
                    .pdftex_fail("invalid format (it should not have happened)"),
            }
            self.put_ushort(ce.platform_id as i64);
            self.put_ushort(ce.encoding_id as i64);
            self.put_ulong(ce.offset as i64);
        }
        for ce in cmaps.iter() {
            match ce.format {
                0 => self.byte_encoding(),
                4 => self.seg_map_delta(),
                6 => self.trimmed_table_map(),
                _ => {}
            }
        }
        self.set_chksm(tab);
    }

    /// `prepend_subset_tags`: the subset tag and `+`, as the record's
    /// platform writes its strings.
    fn prepend_subset_tags(&self, index: usize, out: &mut Vec<u8>) -> usize {
        let tag = self.fd.subset_tag.unwrap_or(*b"AAAAAA");
        if self.name_tab[index].platform_id == 3 {
            for t in tag {
                out.push(0);
                out.push(t);
            }
            out.push(0);
            out.push(b'+');
            14
        } else {
            out.extend_from_slice(&tag);
            out.push(b'+');
            7
        }
    }

    /// `ttf_write_name`.
    fn write_name(&mut self) {
        let tab = self.name_lookup(b"name", true).unwrap();
        let new_name_buf = if self.subsetted {
            let mut buf = Vec::new();
            for i in 0..self.name_tab.len() {
                let n = self.name_tab[i];
                self.name_tab[i].new_offset = buf.len() as u16;
                let l = if (n.name_id == 1 || n.name_id == 3 || n.name_id == 4 || n.name_id == 6)
                    && ((n.platform_id == 1 && n.encoding_id == 0)
                        || (n.platform_id == 3 && n.encoding_id == 0)
                        || (n.platform_id == 3 && n.encoding_id == 1))
                {
                    self.prepend_subset_tags(i, &mut buf)
                } else {
                    0
                };
                let bytes = self.name_bytes(n.offset as usize, n.length as usize);
                buf.extend_from_slice(&bytes);
                self.name_tab[i].new_length = n.length.wrapping_add(l as u16);
            }
            buf
        } else {
            self.name_buf.clone()
        };
        self.reset_chksm(tab);
        self.put_ushort(0); // Format selector
        let n = self.name_tab.len() as i64;
        self.put_ushort(n);
        self.put_ushort(3 * 2 + n * 6 * 2);
        for i in 0..self.name_tab.len() {
            let r = self.name_tab[i];
            self.put_ushort(r.platform_id as i64);
            self.put_ushort(r.encoding_id as i64);
            self.put_ushort(r.language_id as i64);
            self.put_ushort(r.name_id as i64);
            self.put_ushort(r.new_length as i64);
            self.put_ushort(r.new_offset as i64);
        }
        for b in new_name_buf {
            self.put_char(b as i8 as i64);
        }
        self.set_chksm(tab);
    }

    /// `ttf_write_dirtab`: the table directory, and the font's
    /// `checkSumAdjustment`.
    fn write_dirtab(&mut self) {
        let save_offset = self.offset();
        self.seek_outbuf(TABDIR_OFF as u32);
        if self.subsetted {
            for name in NEWTABNAMES {
                let Some(tab) = self.name_lookup(name, false) else {
                    continue;
                };
                let t = self.dir_tab[tab];
                for k in 0..4 {
                    self.put_char(t.tag[k] as i8 as i64);
                }
                self.put_ulong(t.checksum as i64);
                self.put_ulong(t.offset as i64);
                self.put_ulong(t.length as i64);
            }
        } else {
            for tab in 0..self.dir_tab.len() {
                let t = self.dir_tab[tab];
                for k in 0..4 {
                    self.put_char(t.tag[k] as i8 as i64);
                }
                self.put_ulong(t.checksum as i64);
                self.put_ulong(t.offset as i64);
                self.put_ulong(t.length as i64);
            }
        }
        // adjust checkSumAdjustment
        self.tmp_ulong = 0;
        self.checksum = 0;
        let mut i: u32 = 0;
        while i < save_offset {
            // fb_array is `char *`: the byte as the platform's `char`
            let b = self.fb.get(i as usize).copied().unwrap_or(0) as std::ffi::c_char as i32;
            self.tmp_ulong = (self.tmp_ulong << 8).wrapping_add(b as u32);
            i += 1;
            if i % 4 == 0 {
                self.checksum = self.checksum.wrapping_add(self.tmp_ulong);
                self.tmp_ulong = 0;
            }
        }
        if i % 4 != 0 {
            self.g
                .pdftex_warn(&format!("font length is not a multiple of 4 ({i})"));
            self.checksum = self.checksum.wrapping_shl(8 * (4 - i % 4));
        }
        let k = 0xB1B0AFBAu32.wrapping_sub(self.checksum);
        self.seek_outbuf(self.check_sum_adjustment_offset);
        self.put_ulong(k as i64);
        self.seek_outbuf(save_offset);
    }

    /// `ttf_write_glyf`: the used glyphs, in their new order; the
    /// components of a composite glyph are appended to the order as they
    /// are met.
    fn write_glyf(&mut self) {
        let tab = self.name_lookup(b"glyf", true).unwrap();
        let glyf_offset = self.dir_tab[tab].offset as i64;
        let new_glyf_offset = self.offset() as i64;
        self.reset_chksm(tab);
        let mut idn = 0usize;
        while idn < self.new_glyphs_count as usize {
            let id = self.glyph_index[idn] as usize;
            self.glyph_tab[id].newoffset = (self.offset() as i64 - new_glyf_offset) as i32;
            let (o, o1) = (
                self.glyph_tab[id].offset,
                self.glyph_tab.get(id + 1).map_or(0, |g| g.offset),
            );
            if o != o1 {
                self.seek_off(glyf_offset + o as i64);
                let k = self.copy_short();
                self.ncopy(4 * 2);
                if k < 0 {
                    let mut flags;
                    loop {
                        flags = self.copy_ushort();
                        let idx = self.get_ushort() as usize;
                        if idx >= self.glyph_tab.len() {
                            // (C indexes past glyph_tab)
                            self.g.pdftex_fail(&format!(
                                "glyph index {idx} out of range in composite glyph"
                            ));
                        }
                        if self.glyph_tab[idx].newindex < 0 {
                            self.glyph_tab[idx].newindex = self.new_glyphs_count as i16;
                            let n = self.new_glyphs_count as usize;
                            if n < self.glyph_index.len() {
                                self.glyph_index[n] = idx as i64;
                            } else {
                                self.glyph_index.push(idx as i64);
                            }
                            // N.B.: this changes `new_glyphs_count', which
                            // appears in the condition of the loop
                            self.new_glyphs_count = self.new_glyphs_count.wrapping_add(1);
                        }
                        let ni = self.glyph_tab[idx].newindex as i64;
                        self.put_ushort(ni);
                        if flags & ARG_1_AND_2_ARE_WORDS != 0 {
                            self.ncopy(2 * 2);
                        } else {
                            self.ncopy(2);
                        }
                        if flags & WE_HAVE_A_SCALE != 0 {
                            self.ncopy(2);
                        } else if flags & WE_HAVE_AN_X_AND_Y_SCALE != 0 {
                            self.ncopy(2 * 2);
                        } else if flags & WE_HAVE_A_TWO_BY_TWO != 0 {
                            self.ncopy(4 * 2);
                        }
                        if flags & MORE_COMPONENTS == 0 {
                            break;
                        }
                    }
                    if flags & WE_HAVE_INSTRUCTIONS != 0 {
                        let n = self.copy_ushort() as i64;
                        self.ncopy(n);
                    }
                } else {
                    self.ncopy(o1 as i64 - o as i64 - 2 - 4 * 2);
                }
            }
            idn += 1;
        }
        self.last_glyf_offset = (self.offset() as i64 - new_glyf_offset) as u32;
        self.set_chksm(tab);
    }

    /// `ttf_reindex_glyphs`: the new glyph index of each character.
    fn reindex_glyphs(&mut self, st: &mut Fonts) {
        let mut cmap: Option<Vec<i32>> = None;
        let mut cmap_not_found = false;
        for ei in 0..256usize {
            self.ttfenc_tab[ei].newindex = 0; // index of ".notdef" glyph
            let glyph: usize;
            'found: {
                // handle case of subfonts first
                if self.subfont.is_some() {
                    let code = self.ttfenc_tab[ei].code;
                    if code == -1 {
                        break 'found;
                    }
                    if cmap.is_none() && !cmap_not_found {
                        let (pid, eid) = (self.pid, self.eid);
                        cmap = self.read_cmap(st, pid, eid, true);
                        if cmap.is_none() {
                            cmap_not_found = true;
                        }
                    }
                    let Some(t) = &cmap else {
                        break 'found;
                    };
                    let v = t.get(code as usize).copied().unwrap_or(-1);
                    if v < 0 {
                        let tfm = String::from_utf8_lossy(&self.tfm_name).into_owned();
                        self.g.pdftex_warn(&format!(
                            "subfont {tfm}: wrong mapping: character {ei} --> 0x{code:04X} --> .notdef"
                        ));
                        break 'found;
                    }
                    glyph = v as usize;
                    self.append_new_glyph(ei, glyph);
                    break 'found;
                }
                // handle case of reencoded fonts
                let name = self.ttfenc_tab[ei].name.clone();
                if name.as_slice() == NOTDEF {
                    break 'found;
                }
                // look up by name first
                let count = self.glyphs_count as usize;
                if let Some(gi) = self.glyph_tab[..count]
                    .iter()
                    .position(|g| g.name.as_slice() != NOTDEF && g.name == name)
                {
                    self.append_new_glyph(ei, gi);
                    break 'found;
                }
                // scan form `uniABCD'
                let (index, n) = super::cfmt::scan_prefixed(&name, c"uni%X%n");
                if n == name.len() as i32 {
                    if cmap.is_none() && !cmap_not_found {
                        // need to read the unicode mapping, ie (pid,eid) = (3,1) or (0,3)
                        cmap = self.read_cmap(st, 3, 1, false);
                        if cmap.is_none() {
                            cmap = self.read_cmap(st, 0, 3, false);
                        }
                        if cmap.is_none() {
                            self.g.pdftex_warn(
                                "no unicode mapping found, all `uniXXXX' names will be ignored",
                            );
                            cmap_not_found = true; // once only
                        }
                    }
                    let Some(t) = &cmap else {
                        break 'found;
                    };
                    // (C reads past the table for an index above 0xFFFF)
                    let v = t.get(index as u32 as usize).copied().unwrap_or(-1);
                    if v != -1 {
                        if v >= self.glyphs_count as i32 {
                            let mut msg = b"`".to_vec();
                            msg.extend_from_slice(&name);
                            msg.extend_from_slice(
                                format!(
                                    "' is mapped to index {v} which is out of valid range [0..{})",
                                    self.glyphs_count
                                )
                                .as_bytes(),
                            );
                            self.g.pdftex_warn_bytes(&msg);
                            break 'found;
                        }
                        self.append_new_glyph(ei, v as usize);
                    } else {
                        self.g.pdftex_warn(&format!(
                            "`unicode uni{:04X}' is not mapped to any glyph",
                            index as u32
                        ));
                    }
                    break 'found;
                }
                // scan form `index123'
                let (index, n) = super::cfmt::scan_prefixed(&name, c"index%i%n");
                if n == name.len() as i32 {
                    if index >= self.glyphs_count as i32 {
                        let mut msg = b"`".to_vec();
                        msg.extend_from_slice(&name);
                        msg.extend_from_slice(
                            format!("' out of valid range [0..{})", self.glyphs_count).as_bytes(),
                        );
                        self.g.pdftex_warn_bytes(&msg);
                        break 'found;
                    }
                    // (a negative index is outside glyph_tab in C too)
                    if index >= 0 {
                        self.append_new_glyph(ei, index as usize);
                    }
                    break 'found;
                }
                // not found
                let mut msg = b"glyph `".to_vec();
                msg.extend_from_slice(&name);
                msg.extend_from_slice(b"' not found");
                self.g.pdftex_warn_bytes(&msg);
            }
        }
    }

    /// `append_new_glyph`.
    fn append_new_glyph(&mut self, ei: usize, glyph: usize) {
        if self.glyph_tab[glyph].newindex < 0 {
            let n = self.new_glyphs_count as usize;
            if n < self.glyph_index.len() {
                self.glyph_index[n] = glyph as i64;
            } else {
                self.glyph_index.push(glyph as i64);
            }
            self.glyph_tab[glyph].newindex = self.new_glyphs_count as i16;
            self.new_glyphs_count = self.new_glyphs_count.wrapping_add(1);
        }
        self.ttfenc_tab[ei].newindex = self.glyph_tab[glyph].newindex as i64;
    }

    /// `ttf_write_head`.
    fn write_head(&mut self) {
        let tab = self.seek_tab(b"head", 0);
        self.reset_chksm(tab);
        self.ncopy(2 * 4);
        self.check_sum_adjustment_offset = self.offset();
        self.put_ulong(0);
        self.skip(4); // skip checkSumAdjustment
        self.ncopy(4 + 2 * 2 + 16 + 4 * 2 + 2 * 2 + 2);
        if self.subsetted {
            let lf = self.loca_format as i64;
            self.put_short(lf);
            self.put_short(0);
        } else {
            self.ncopy(2 * 2);
        }
        self.set_chksm(tab);
    }

    /// `ttf_write_hhea`.
    fn write_hhea(&mut self) {
        let tab = self.seek_tab(b"hhea", 0);
        self.reset_chksm(tab);
        self.ncopy(4 + 3 * 2 + 2 + 3 * 2 + 8 * 2);
        let n = self.new_glyphs_count as i64;
        self.put_ushort(n);
        self.set_chksm(tab);
    }

    /// `ttf_write_htmx`.
    fn write_htmx(&mut self) {
        let tab = self.seek_tab(b"hmtx", 0);
        self.reset_chksm(tab);
        for idn in 0..self.new_glyphs_count as usize {
            let g = &self.glyph_tab[self.glyph_index[idn] as usize];
            let (w, l) = (g.adv_width as i64, g.lsb as i64);
            self.put_ufword(w);
            self.put_ufword(l);
        }
        self.set_chksm(tab);
    }

    /// `ttf_write_loca`.
    fn write_loca(&mut self) {
        let tab = self.seek_tab(b"loca", 0);
        self.reset_chksm(tab);
        self.loca_format = 0;
        let n = self.new_glyphs_count as usize;
        if self.last_glyf_offset >= 0x00020000 || (self.last_glyf_offset & 1) != 0 {
            self.loca_format = 1;
        } else {
            for idn in 0..n {
                if self.glyph_tab[self.glyph_index[idn] as usize].newoffset & 1 != 0 {
                    self.loca_format = 1;
                    break;
                }
            }
        }
        if self.loca_format != 0 {
            for idn in 0..n {
                let o = self.glyph_tab[self.glyph_index[idn] as usize].newoffset as i64;
                self.put_ulong(o);
            }
            let l = self.last_glyf_offset as i64;
            self.put_ulong(l);
        } else {
            for idn in 0..n {
                let o = self.glyph_tab[self.glyph_index[idn] as usize].newoffset / 2;
                self.put_ushort(o as i64);
            }
            let l = (self.last_glyf_offset / 2) as i64;
            self.put_ushort(l);
        }
        self.set_chksm(tab);
    }

    /// `ttf_write_mapx`.
    fn write_mapx(&mut self) {
        let tab = self.seek_tab(b"maxp", 4 + 2);
        self.reset_chksm(tab);
        self.put_fixed(0x00010000);
        let n = self.new_glyphs_count as i64;
        self.put_ushort(n);
        self.ncopy(13 * 2);
        self.set_chksm(tab);
    }

    /// `ttf_write_OS2`.
    fn write_os2(&mut self) {
        let tab = self.seek_tab(b"OS/2", 0);
        self.reset_chksm(tab);
        let version = self.get_ushort();
        if version > 5 {
            self.g.pdftex_warn(&format!(
                "unknown version of OS/2 table ({version:04X})"
            ));
        }
        self.put_ushort(0x0001); // fix version to 1
        self.ncopy(2 * 2 + 13 * 2 + 10);
        self.skip(4 * 4); // ulUnicodeRange 1--4
        self.put_ulong(0x00000003); // Basic Latin + Latin-1 Supplement (0x0000--0x00FF)
        self.put_ulong(0x10000000); // Private Use (0xE000--0xF8FF)
        self.put_ulong(0x00000000);
        self.put_ulong(0x00000000);
        self.ncopy(4 + 2); // achVendID + fsSelection
        self.skip(2 * 2);
        self.put_ushort(0x0000); // usFirstCharIndex
        self.put_ushort(0xF0FF); // usLastCharIndex
        self.ncopy(5 * 2);
        // for version 0 the OS/2 table ends here, the rest is for version 1
        self.put_ulong(0x80000000); // Symbol Character Set---don't use any code page
        self.put_ulong(0x00000000);
        self.set_chksm(tab);
    }

    /// `ttf_write_post`.
    fn write_post(&mut self) {
        let tab = self.seek_tab(b"post", 4);
        self.reset_chksm(tab);
        if !self.fd.write_ttf_glyph_names || self.post_format == 0x00030000 {
            self.put_fixed(0x00030000);
            self.ncopy(4 + 2 * 2 + 5 * 4);
        } else {
            self.put_fixed(0x00020000);
            self.ncopy(4 + 2 * 2 + 5 * 4);
            let n = self.new_glyphs_count as usize;
            self.put_ushort(n as i64);
            let mut k: u32 = 0;
            for idn in 0..n {
                let g = self.glyph_index[idn] as usize;
                let e = &mut self.glyph_tab[g];
                if e.name_index as usize >= NMACGLYPHS || unsafe_name(&e.name) {
                    e.name_index = (NMACGLYPHS as u32 + k) as u16;
                    k += 1;
                }
                let ni = e.name_index as i64;
                self.put_ushort(ni);
            }
            for idn in 0..n {
                let g = self.glyph_index[idn] as usize;
                if self.glyph_tab[g].name_index as usize >= NMACGLYPHS {
                    let s = self.glyph_tab[g].name.clone();
                    self.put_byte(s.len() as i64);
                    for b in s {
                        self.put_char(b as i8 as i64);
                    }
                }
            }
        }
        self.set_chksm(tab);
    }

    /// `ttf_init_font`: the offset table, room for `n` directory entries.
    fn init_font(&mut self, n: u16) {
        let n = n as i64;
        let (mut i, mut k) = (1i64, 0i64);
        while i <= n {
            i <<= 1;
            k += 1;
        }
        self.put_fixed(0x00010000); // font version
        self.put_ushort(n); // number of tables
        self.put_ushort(i << 3); // search range
        self.put_ushort(k - 1); // entry selector
        self.put_ushort((n << 4) - (i << 3)); // range shift
        self.seek_outbuf((TABDIR_OFF as i64 + n * 4 * 4) as u32);
    }

    /// `ttf_subset_font`.
    fn subset_font(&mut self, st: &mut Fonts) {
        let n = self.new_ntabs;
        self.init_font(n);
        for t in [b"PCLT", b"fpgm", b"cvt ", b"prep"] {
            if self.name_lookup(t, false).is_some() {
                self.copytab(t);
            }
        }
        self.reindex_glyphs(st);
        self.write_glyf();
        self.write_loca();
        self.write_os2();
        self.write_head();
        self.write_hhea();
        self.write_htmx();
        self.write_mapx();
        self.write_name();
        self.write_post();
        self.write_cmap();
        self.write_dirtab();
    }

    /// `ttf_copy_font`.
    fn copy_font(&mut self) {
        let n = self.ntabs;
        self.init_font(n);
        for tab in 0..self.dir_tab.len() {
            let tag = self.dir_tab[tab].tag;
            if &tag == b"head" {
                self.write_head();
            } else {
                self.copytab(&tag);
            }
        }
        self.write_dirtab();
    }
}

/// `strip_spaces_and_delims`: a name without PostScript delimiters and
/// white space.
fn strip_spaces_and_delims(s: &[u8]) -> Vec<u8> {
    let mut out: Vec<u8> = s
        .iter()
        .copied()
        .filter(|&c| {
            !matches!(
                c,
                b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
            ) && !matches!(c, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
        })
        .collect();
    // xstrdup of the buffer: it ends at the first NUL
    if let Some(z) = out.iter().position(|&b| b == 0) {
        out.truncate(z);
    }
    out
}

/// `unsafe_name`.
fn unsafe_name(s: &[u8]) -> bool {
    AMBIGUOUS_NAMES.contains(&s)
}

impl Globals {
    fn new_ttf<'a>(&'a mut self, fd: &'a mut FdEntry, st: &Fonts, file: CFile) -> Ttf<'a> {
        let fm = st.map.fms[fd.fm].clone().expect("live map entry");
        let enc_glyph_names = fd.fe.map(|fe| st.enc.fes[fe].glyph_names.clone());
        let fb = super::with_state(|s| std::mem::take(&mut s.out.fb));
        let fb_pos = fb.len();
        Ttf {
            g: self,
            file,
            fb,
            fb_pos,
            ntabs: 0,
            upem: 0,
            post_format: 0,
            loca_format: 0,
            last_glyf_offset: 0,
            glyphs_count: 0,
            new_glyphs_count: 2,
            nhmtxs: 0,
            new_ntabs: DEFAULT_NTABS,
            glyph_tab: Vec::new(),
            no_glyphs: true,
            glyph_index: Vec::new(),
            name_tab: Vec::new(),
            name_buf: Vec::new(),
            dir_tab: Vec::new(),
            checksum: 0,
            tab_length: 0,
            tmp_ulong: 0,
            check_sum_adjustment_offset: 0,
            ttfenc_tab: vec![TtfEnc::default(); 256],
            fd,
            ff_name: fm.ff_name.clone().unwrap_or_default(),
            subsetted: fm.is_subsetted(),
            subfont: fm.subfont.clone(),
            pid: fm.pid as i32,
            eid: fm.eid as i32,
            tfm_name: fm.tfm_name.clone(),
            enc_glyph_names,
        }
    }

    /// Open the font file of `fd`'s map entry (`set_cur_file_name` and
    /// `open_input`); fails the run with `msg` if it cannot be read.
    fn open_font_file(&mut self, st: &Fonts, fd: &FdEntry, format: Format, msg: &str) -> (String, CFile) {
        let fm = st.map.fms[fd.fm].as_ref().expect("live map entry");
        let ff = fm.ff_name.clone().unwrap_or_default();
        set_cur_file_name(Some(&ff));
        let found = self.open_input_named(&ff, format);
        let Some((path, file)) = found.and_then(|p| CFile::open(p.as_bytes()).map(|f| (p, f))) else {
            self.pdftex_fail(msg);
        };
        set_cur_file_name(Some(path.as_bytes()));
        (path, file)
    }

    /// `writettf` (writettf.c): embed the TrueType font of `fd` into the
    /// font buffer; the font's length (`ttf_length`).
    pub fn writettf(&mut self, st: &mut Fonts, fd: &mut FdEntry) -> i32 {
        let fm = st.map.fms[fd.fm].clone().expect("live map entry");
        set_cur_file_name(fm.ff_name.as_deref());
        if fm.is_subsetted() && fd.fe.is_none() && fm.subfont.is_none() {
            self.pdftex_fail("Subset TrueType must be a reencoded or a subfont");
        }
        let (path, mut file) = self.open_font_file(
            st,
            fd,
            Format::TrueType,
            "cannot open TrueType font file for reading",
        );
        // skip ttc header, prepare for reading first font
        if path.len() >= 4 && path[path.len() - 4..].eq_ignore_ascii_case(".ttc") {
            let mut tag = 0u32;
            for _ in 0..4 {
                let c = file.getc();
                if c < 0 {
                    self.pdftex_fail("unexpected EOF");
                }
                tag = (tag << 8) | c as u32;
            }
            if tag != 0x74746366 {
                // ttcf
                file.seek(0, Whence::Set);
            } else {
                let mut v = [0u8; 12];
                if file.read(&mut v) < 12 {
                    self.pdftex_fail("unexpected EOF");
                }
                // ignore the version and numFonts; go to the first font
                let off = u32::from_be_bytes([v[8], v[9], v[10], v[11]]);
                file.seek(off as i64, Whence::Set);
            }
        }
        if fm.is_subsetted() {
            self.tex_printf(format!("<{path}").as_bytes());
        } else {
            self.tex_printf(format!("<<{path}").as_bytes());
        }
        fd.ff_found = true;
        let mut t = self.new_ttf(fd, st, file);
        t.read_font();
        // pdfsaveoffset = pdfoffset(); pdfflush();
        t.g.pdf_save_offset = t.g.pdf_gone + t.g.pdf_ptr as i64;
        t.g.pdf_flush();
        if t.subsetted {
            t.copy_encoding();
            t.subset_font(st);
        } else {
            t.copy_font();
        }
        let ttf_length = t.offset() as i32;
        let mut fb = std::mem::take(&mut t.fb);
        fb.truncate(t.fb_pos); // fb_flush writes up to fb_ptr
        drop(t);
        super::with_state(|s| s.out.fb = fb);
        if fm.is_subsetted() {
            self.tex_printf(b">");
        } else {
            self.tex_printf(b">>");
        }
        set_cur_file_name(None);
        ttf_length
    }

    /// `writeotf` (writettf.c): embed the `CFF ` table of the OpenType font
    /// of `fd` into the font buffer, whole.
    pub fn writeotf(&mut self, st: &mut Fonts, fd: &mut FdEntry) {
        let fm = st.map.fms[fd.fm].clone().expect("live map entry");
        set_cur_file_name(fm.ff_name.as_deref());
        if fm.is_subsetted() {
            self.pdftex_fail("OTF fonts must be included entirely");
        }
        let (path, file) = self.open_font_file(
            st,
            fd,
            Format::OpenType,
            "cannot open OpenType font file for reading",
        );
        self.tex_printf(format!("<<{path}").as_bytes());
        fd.ff_found = true;
        let mut t = self.new_ttf(fd, st, file);
        t.read_tabdir();
        // read font parameters
        if t.name_lookup(b"head", false).is_some() {
            t.read_head();
        }
        if t.name_lookup(b"hhea", false).is_some() {
            t.read_hhea();
        }
        if t.name_lookup(b"PCLT", false).is_some() {
            t.read_pclt();
        }
        if t.name_lookup(b"post", false).is_some() {
            t.read_post();
        }
        // copy font file
        let tab = t.seek_tab(b"CFF ", 0);
        for _ in 0..t.dir_tab[tab].length {
            t.copy_char();
        }
        let fb = std::mem::take(&mut t.fb);
        drop(t);
        super::with_state(|s| s.out.fb = fb);
        self.tex_printf(b">>");
        set_cur_file_name(None);
    }
}
