//! `writet3.c` and `pkin.c`, ported: Type 3 fonts from PK bitmaps (or from
//! pdfTeX's own `.pgc` glyph files).
//!
//! pdfTeX writes a font as Type 3 when it has no map entry, or when its map
//! entry is a bitmap font (no PostScript name, no font file). The glyphs
//! then come from a `.pgc` file if there is one (`kpse_miscfonts_format`),
//! else from the font's PK file at the device resolution
//! (`\pdfpkresolution`), which kpathsea finds or mktexpk makes
//! (`kpse_find_pk`, linked: [`crate::system::find_pk`]).
//!
//! Every C function here names its original. The C globals of the two
//! files live in [`T3`] for the length of one `writet3` call (pdfTeX resets
//! them at its start); `pkin.c`'s unpacking state is [`Pk`].

use super::cfmt;
use super::fonts::{Fonts, GlyphNames, NOTDEF};
use super::output::set_cur_file_name;
use crate::generated::Globals;
use crate::resolver::Format;
use std::collections::BTreeSet;

/// `T3_BUF_SIZE`.
const T3_BUF_SIZE: usize = 1024;

/// `chardesc` (ptexlib.h): one character of a PK file, its raster as
/// 16-bit words (`halfword`s), each row starting on a word.
#[derive(Default)]
pub struct CharDesc {
    pub charcode: i32,
    pub cwidth: i32,
    pub cheight: i32,
    pub xoff: i32,
    pub yoff: i32,
    pub xescape: i32,
    pub raster: Vec<i32>,
}

/// The state of one `writet3` call (writet3.c's file-level statics).
struct T3 {
    /// `t3_char_procs`: the object of each glyph's procedure, 0 for none.
    char_procs: [i32; 256],
    /// `t3_char_widths` (C `float`s).
    char_widths: [f32; 256],
    /// `t3_glyph_num`.
    glyph_num: i32,
    /// `t3_font_scale` (a C `float`).
    font_scale: f32,
    /// `t3_b0`..`t3_b3`: the font's bounding box.
    b: [i32; 4],
    /// `t3_image_used`.
    image_used: bool,
    /// `is_pk_font`.
    is_pk_font: bool,
}

impl T3 {
    fn new() -> T3 {
        T3 {
            char_procs: [0; 256],
            char_widths: [0.0; 256],
            glyph_num: 0,
            font_scale: 0.0,
            b: [0; 4],
            image_used: false,
            is_pk_font: false,
        }
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

/// A binary file read with `getc`/`feof` (`t3_file`): read whole when it
/// is opened, as the C library would buffer it.
struct T3File {
    data: Vec<u8>,
    pos: usize,
    eof: bool,
}

impl T3File {
    fn new(data: Vec<u8>) -> T3File {
        T3File {
            data,
            pos: 0,
            eof: false,
        }
    }

    /// `xgetc`: the next byte, or -1 (`EOF`) at the end, which sets the
    /// end-of-file flag.
    fn getc(&mut self) -> i32 {
        match self.data.get(self.pos) {
            Some(&b) => {
                self.pos += 1;
                b as i32
            }
            None => {
                self.eof = true;
                -1
            }
        }
    }
}

/// The line reader of `.pgc` files (`t3_line_array`, `t3_getline`).
struct Lines {
    line: Vec<u8>,
}

impl Lines {
    /// `t3_prefix(s)`: whether the line starts with `s`.
    fn prefix(&self, s: &[u8]) -> bool {
        self.line.starts_with(s)
    }
}

impl Globals {
    /// `t3_getline`: the next line that is not a comment and not empty,
    /// through `append_char_to_buf` and `append_eol` (ptexmac.h).
    fn t3_getline(&mut self, f: &mut T3File, l: &mut Lines) {
        loop {
            l.line.clear();
            let mut c = f.getc();
            while !f.eof {
                // append_char_to_buf(c, t3_line_ptr, t3_line_array, t3_line_limit):
                // the array grows, so the size check never fails
                if c == 9 {
                    c = 32;
                }
                if c == 13 || c == -1 {
                    c = 10;
                }
                if c != b' ' as i32 || (!l.line.is_empty() && *l.line.last().unwrap() != 32) {
                    l.line.push(c as u8);
                }
                if c == 10 {
                    break;
                }
                c = f.getc();
            }
            // append_eol(t3_line_ptr, t3_line_array, T3_BUF_SIZE)
            if l.line.len() + 2 > T3_BUF_SIZE {
                self.pdftex_fail("buffer overflow at file writet3.c, line 66");
            }
            let n = l.line.len();
            if n > 1 && l.line[n - 1] != 10 {
                l.line.push(10);
            }
            let n = l.line.len();
            if n > 2 && l.line[n - 2] == 32 {
                l.line[n - 2] = 10;
                l.line.pop();
            }
            if (l.line.len() < 2 || l.line.first() == Some(&b'%')) && !f.eof {
                continue;
            }
            return;
        }
    }

    /// `t3_putline`.
    fn t3_putline(&mut self, l: &Lines) {
        for &b in &l.line {
            self.c_pdf_out(b);
        }
    }

    /// `t3_check_eof`.
    fn t3_check_eof(&mut self, f: &T3File) {
        if f.eof {
            self.pdftex_fail("unexpected end of file");
        }
    }

    /// `t3_write_glyph`: one glyph of a `.pgc` file.
    fn t3_write_glyph(&mut self, t: &mut T3, file: &mut T3File, l: &mut Lines, f: i32) {
        const BEGIN: &[u8] = b"\\pdfglyph";
        const END: &[u8] = b"\\endglyph";
        self.t3_getline(file, l);
        if !l.prefix(BEGIN) {
            return;
        }
        // sscanf(t3_line_array + strlen(t3_begin_glyph_str) + 1, "%i %i %i %i %i %i %i %i =", ...)
        let rest = l.line.get(BEGIN.len() + 1..).unwrap_or(&[]);
        let (n, v) = cfmt::scan_ints8(rest, c"%i %i %i %i %i %i %i %i =");
        if n != 8 {
            let mut shown = l.line.clone();
            if shown.last() == Some(&10) {
                shown.pop();
            }
            self.pdftex_fail(&format!(
                "invalid glyph preamble: `{}'",
                String::from_utf8_lossy(&shown)
            ));
        }
        let [glyph_index, width, _height, _depth, llx, lly, urx, ury] = v;
        if glyph_index < self.font_bc[f as usize] || glyph_index > self.font_ec[f as usize] {
            return;
        }
        if !self.pdf_char_marked(f, glyph_index) {
            while !l.prefix(END) {
                self.t3_check_eof(file);
                self.t3_getline(file, l);
            }
            return;
        }
        t.update_bbox(llx, lly, urx, ury, t.glyph_num == 0);
        t.glyph_num += 1;
        self.pdf_new_dict(0, 0, 0);
        t.char_procs[glyph_index as usize] = self.obj_ptr;
        if width == 0 {
            let w = self.get_charwidth(f, glyph_index);
            t.char_widths[glyph_index as usize] =
                (w as f32 / t.font_scale) / self.pdf_font_size[f as usize] as f32;
        } else {
            t.char_widths[glyph_index as usize] = width as f32;
        }
        self.pdf_begin_stream();
        self.t3_getline(file, l);
        self.pdf_printf(
            format!(
                "{} 0 {} {} {} {} d1\nq\n",
                t.char_widths[glyph_index as usize] as i32,
                llx,
                lly,
                urx,
                ury
            )
            .as_bytes(),
        );
        while !l.prefix(END) {
            self.t3_check_eof(file);
            if l.prefix(b"BI") {
                t.image_used = true;
            }
            self.t3_putline(l);
            self.t3_getline(file, l);
        }
        self.pdf_puts(b"Q\n");
        self.pdf_end_stream();
    }

    /// `get_pk_font_scale`.
    fn get_pk_font_scale(&mut self, f: i32) -> i32 {
        let size = self.pdf_font_size[f as usize];
        let (a, b) = (self.one_hundred_bp, self.fixed_decimal_digits + 2);
        let s = self.divide_scaled(size, a, b);
        let p = self.pk_scale_factor;
        self.divide_scaled(p, s, 0)
    }

    /// `pk_char_width`.
    fn pk_char_width(&mut self, f: i32, w: i32) -> i32 {
        let size = self.pdf_font_size[f as usize];
        let a = self.divide_scaled(w, size, 7);
        let b = self.get_pk_font_scale(f);
        self.divide_scaled(a, b, 0)
    }

    /// `getpkcharwidth` (writet3.c): the width of a character of bitmap
    /// font `f` as the PDF will advance it.
    pub fn get_pk_char_width(&mut self, f: i32, w: i32) -> i32 {
        let scale = self.get_pk_font_scale(f) as f64;
        let cw = self.pk_char_width(f, w) as f64;
        ((scale / 100000.0) * (cw / 100.0) * self.pdf_font_size[f as usize] as f64) as i32
    }

    /// `writepk`: the glyphs of font `f` from its PK file.
    fn writepk(&mut self, t: &mut T3, f: i32) -> Option<T3File> {
        let fr = self.fixed_pk_resolution;
        let ratio = self.pdf_font_size[f as usize] as f32 / self.font_dsize[f as usize] as f32;
        let dpi = (fr as f32 * ratio) as f64;
        let dpi = magstep_fix(dpi.round() as u32, fr as u32);
        let name = self.c_string(self.font_name[f as usize]);
        set_cur_file_name(Some(&name));
        let found = crate::system::find_pk(&String::from_utf8_lossy(&name), dpi);
        let found = match found {
            Some(g) if g.name == name && bitmap_tolerance(g.dpi as f32 as f64, dpi as f32 as f64) => g,
            _ => self.pdftex_fail(&format!(
                "Font {} at {} not found",
                String::from_utf8_lossy(&name),
                dpi as i32
            )),
        };
        let path = found.path.to_string_lossy().into_owned();
        let Ok(data) = std::fs::read(&found.path) else {
            // xfopen
            let prog = crate::system::invocation_name();
            eprintln!("{prog}: fopen({path}) failed");
            crate::system::exit_process(self, 1);
        };
        let mut file = T3File::new(data);
        t.image_used = true;
        t.is_pk_font = true;
        self.tex_printf(format!(" <{path}").as_bytes());
        let mut cd = CharDesc::default();
        let mut pk = Pk::default();
        let mut check_preamble = true;
        while self.readchar(&mut pk, &mut file, check_preamble, &mut cd) {
            check_preamble = false;
            let c = cd.charcode;
            // (C reads past `pdfcharused` for a code above 255, which no
            // PK file for a TFM font has)
            if !(0..256).contains(&c) || !self.pdf_char_marked(f, c) {
                continue;
            }
            let cu = c as u8 as usize; // charcode is below 256 for a marked character
            let w = self.get_charwidth(f, c);
            t.char_widths[cu] = self.pk_char_width(f, w) as f32;
            let is_null_glyph = if cd.cwidth < 1 || cd.cheight < 1 {
                cd.cwidth = (t.char_widths[cu] as f64 / 100.0).round() as i32;
                cd.xescape = cd.cwidth;
                cd.cheight = 1;
                cd.xoff = 0;
                cd.yoff = 0;
                true
            } else {
                false
            };
            let llx = -cd.xoff;
            let lly = cd.yoff - cd.cheight + 1;
            let urx = cd.cwidth + llx + 1;
            let ury = cd.cheight + lly;
            t.update_bbox(llx, lly, urx, ury, t.glyph_num == 0);
            t.glyph_num += 1;
            self.pdf_new_dict(0, 0, 0);
            t.char_procs[cu] = self.obj_ptr;
            self.pdf_begin_stream();
            self.pdf_print_real(t.char_widths[cu] as i32, 2);
            self.pdf_printf(format!(" 0 {llx} {lly} {urx} {ury} d1\n").as_bytes());
            if !is_null_glyph {
                self.pdf_printf(
                    format!(
                        "q\n{} 0 0 {} {} {} cm\nBI\n",
                        cd.cwidth, cd.cheight, llx, lly
                    )
                    .as_bytes(),
                );
                self.pdf_printf(format!("/W {}\n/H {}\n", cd.cwidth, cd.cheight).as_bytes());
                self.pdf_puts(b"/IM true\n/BPC 1\n/D [1 0]\nID ");
                let cw = (cd.cwidth + 7) / 8;
                let rw = (cd.cwidth + 15) / 16;
                let mut row = 0usize;
                for _ in 0..cd.cheight {
                    for _ in 0..rw - 1 {
                        let v = cd.raster.get(row).copied().unwrap_or(0);
                        self.c_pdf_out((v / 256) as u8);
                        self.c_pdf_out((v % 256) as u8);
                        row += 1;
                    }
                    let v = cd.raster.get(row).copied().unwrap_or(0);
                    self.c_pdf_out((v / 256) as u8);
                    if 2 * rw == cw {
                        self.c_pdf_out((v % 256) as u8);
                    }
                    row += 1;
                }
                self.pdf_puts(b"\nEI\nQ\n");
            }
            self.pdf_end_stream();
        }
        set_cur_file_name(None);
        Some(file)
    }

    /// `remove_duplicate_glyph_names`: a glyph name used twice by an
    /// encoding makes the Type 3 font invalid; every later use becomes
    /// `.notdef`, with a warning.
    fn remove_duplicate_glyph_names(&mut self, g: &mut GlyphNames, encname: &[u8]) {
        let mut seen: BTreeSet<Vec<u8>> = BTreeSet::new();
        for i in 0..256 {
            if g[i] == NOTDEF {
                continue;
            }
            if !seen.contains(&g[i]) {
                seen.insert(g[i].clone());
            } else {
                let mut msg = encname.to_vec();
                msg.extend_from_slice(format!(": duplicate glyph name at position {i}: ").as_bytes());
                msg.extend_from_slice(&g[i]);
                self.pdftex_warn_bytes(&msg);
                g[i] = NOTDEF.to_vec();
            }
        }
    }

    /// `writet3` (writet3.c): font `f` as a Type 3 font, PDF object
    /// `objnum`; `fm` is its map entry, if it has one.
    pub fn writet3(&mut self, st: &mut Fonts, fm: Option<usize>, objnum: i32, f: i32) {
        let mut t = T3::new();
        let fm_e = fm.and_then(|i| st.map.fms[i].clone());
        let fe = match fm_e.as_ref().and_then(|m| m.encname.clone()) {
            Some(enc) => self.get_fe_entry(st, &enc),
            None => None,
        };
        let mut glyph_names: Option<GlyphNames> = None;
        if let Some(fe) = fe {
            let mut g = std::mem::take(&mut st.enc.fes[fe].glyph_names);
            let encname = fm_e.as_ref().unwrap().encname.clone().unwrap();
            self.remove_duplicate_glyph_names(&mut g, &encname);
            st.enc.fes[fe].glyph_names = g.clone();
            glyph_names = Some(g);
        }
        // packfilename(fontname[f], getnullstr(), maketexstring(".pgc"))
        let ext = self.make_tex_string(b".pgc");
        let (n, a) = (self.font_name[f as usize], self.null_str());
        self.pack_file_name(n, a, ext);
        let s = self.make_name_string();
        let pgc_name = self.c_string(s);
        set_cur_file_name(Some(&pgc_name));
        t.is_pk_font = false;
        let file = match self.t3_open() {
            None => self.writepk(&mut t, f),
            Some((path, data)) => {
                let mut file = T3File::new(data);
                self.tex_printf(format!("<{path}").as_bytes());
                let mut l = Lines { line: Vec::new() };
                self.t3_getline(&mut file, &mut l);
                const SCALE: &[u8] = b"\\pdffontscale";
                let scale = if l.prefix(SCALE) {
                    cfmt::scan_float(l.line.get(SCALE.len() + 1..).unwrap_or(&[]))
                } else {
                    None
                };
                match scale {
                    Some(s) if s > 0.0 && s <= 1000.0 => t.font_scale = s,
                    _ => self.pdftex_fail("missing or invalid font scale"),
                }
                while !file.eof {
                    self.t3_write_glyph(&mut t, &mut file, &mut l, f);
                }
                Some(file)
            }
        };
        if file.is_none() {
            set_cur_file_name(None);
            return;
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
        let tounicode_objnum = match fe {
            Some(fe)
                if self.fixed_gen_tounicode > 0
                    && !self.pdf_font_nobuiltin_tounicode[f as usize] =>
            {
                let names = glyph_names.clone().unwrap();
                let encname = st.enc.fes[fe].name.clone();
                let tfm = fm_e.as_ref().unwrap().tfm_name.clone();
                self.write_tounicode(st, &names, &tfm, Some(&encname))
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
        if t.is_pk_font {
            let pk_font_scale = self.get_pk_font_scale(f);
            self.pdf_puts(b"/FontMatrix [");
            self.pdf_print_real(pk_font_scale, 5);
            self.pdf_puts(b" 0 0 ");
            self.pdf_print_real(pk_font_scale, 5);
            self.pdf_puts(b" 0 0]\n");
        } else {
            let g = cfmt::fmt_g(t.font_scale as f64);
            let mut s = b"/FontMatrix [".to_vec();
            s.extend_from_slice(&g);
            s.extend_from_slice(b" 0 0 ");
            s.extend_from_slice(&g);
            s.extend_from_slice(b" 0 0]\n");
            self.pdf_printf(&s);
        }
        self.pdf_printf(
            format!(
                "/FontBBox [ {} {} {} {} ]\n",
                t.b[0], t.b[1], t.b[2], t.b[3]
            )
            .as_bytes(),
        );
        self.pdf_printf(
            format!(
                "/Resources << /ProcSet [ /PDF {}] >>\n",
                if t.image_used { "/ImageB " } else { "" }
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
        if t.is_pk_font {
            for i in first_char..=last_char {
                self.pdf_print_real(t.char_widths[i as usize] as i32, 2);
                self.pdf_puts(b" ");
            }
        } else {
            for i in first_char..=last_char {
                self.pdf_printf(format!("{} ", t.char_widths[i as usize] as i32).as_bytes());
            }
        }
        self.pdf_puts(b"]\n");
        self.pdf_end_obj();
        let name_of = |i: i32| -> Option<Vec<u8>> {
            glyph_names
                .as_ref()
                .map(|g| g[i as usize].clone())
                .filter(|n| n != NOTDEF)
        };
        self.pdf_begin_dict(eptr, 1); // encoding dictionary
        self.pdf_printf(format!("/Type /Encoding\n/Differences [{first_char}").as_bytes());
        let mut is_notdef;
        if t.char_procs[first_char as usize] == 0 {
            self.pdf_printf(b"/.notdef");
            is_notdef = true;
        } else {
            self.put_glyph_name(name_of(first_char), first_char);
            is_notdef = false;
        }
        for i in first_char + 1..=last_char {
            if t.char_procs[i as usize] == 0 {
                if !is_notdef {
                    self.pdf_printf(format!(" {i}/.notdef").as_bytes());
                    is_notdef = true;
                }
            } else {
                if is_notdef {
                    self.pdf_printf(format!(" {i}").as_bytes());
                    is_notdef = false;
                }
                self.put_glyph_name(name_of(i), i);
            }
        }
        self.pdf_puts(b"]\n");
        self.pdf_end_dict();
        self.pdf_begin_dict(cptr, 1); // CharProcs dictionary
        for i in first_char..=last_char {
            let p = t.char_procs[i as usize];
            if p != 0 {
                let mut s = match name_of(i) {
                    Some(n) => [b"/".as_slice(), &n].concat(),
                    None => format!("/a{i}").into_bytes(),
                };
                s.extend_from_slice(format!(" {p} 0 R\n").as_bytes());
                self.pdf_printf(&s);
            }
        }
        self.pdf_end_dict();
        // t3_close()
        drop(file);
        self.tex_printf(b">");
        set_cur_file_name(None);
    }

    /// `pdf_printf("/%s", glyph_names[i])` or `pdf_printf("/a%i", i)`.
    fn put_glyph_name(&mut self, name: Option<Vec<u8>>, i: i32) {
        match name {
            Some(n) => self.pdf_printf(&[b"/".as_slice(), &n].concat()),
            None => self.pdf_printf(format!("/a{i}").as_bytes()),
        }
    }

    /// `t3_open()`: `open_input(&t3_file, kpse_miscfonts_format,
    /// FOPEN_RBIN_MODE)` of the name in `name_of_file`; the path found
    /// (written back into `name_of_file`) and the file's bytes.
    fn t3_open(&mut self) -> Option<(String, Vec<u8>)> {
        let path = self.open_input_path(Format::MiscFonts)?;
        let data = std::fs::read(&path).ok()?;
        Some((path, data))
    }

    /// `pk_init`: pdftex.web's `kpse_init_prog('PDFTEX', resolution,
    /// mode, nil)` and `kpse_set_program_enabled(kpse_pk_format, 1,
    /// kpse_src_compile)`; `pk_mode` is the string of `\pdfpkmode`, or 0.
    pub fn pk_init(&mut self, resolution: i32, pk_mode: i32) {
        let mode = (pk_mode != 0).then(|| self.c_string(pk_mode));
        crate::system::pk_init(resolution as u32, mode.as_deref());
    }
}

/// `kpse_magstep_fix(dpi, bdpi, NULL)` (kpathsea's magstep.c, linked).
fn magstep_fix(dpi: u32, bdpi: u32) -> u32 {
    #[cfg(feature = "kpathsea")]
    {
        extern "C" {
            fn kpathsea_magstep_fix(
                kpse: *mut std::ffi::c_void,
                dpi: std::ffi::c_uint,
                bdpi: std::ffi::c_uint,
                m_ret: *mut std::ffi::c_int,
            ) -> std::ffi::c_uint;
        }
        // SAFETY: kpathsea_magstep_fix does not use its instance argument
        // ("currenty not used") and accepts a NULL m_ret.
        unsafe { kpathsea_magstep_fix(std::ptr::null_mut(), dpi, bdpi, std::ptr::null_mut()) }
    }
    #[cfg(not(feature = "kpathsea"))]
    {
        let _ = bdpi;
        dpi
    }
}

/// `kpse_bitmap_tolerance(dpi1, dpi2)` (kpathsea's tex-glyph.c, linked).
fn bitmap_tolerance(dpi1: f64, dpi2: f64) -> bool {
    #[cfg(feature = "kpathsea")]
    {
        extern "C" {
            fn kpathsea_bitmap_tolerance(
                kpse: *mut std::ffi::c_void,
                dpi1: f64,
                dpi2: f64,
            ) -> std::ffi::c_int;
        }
        // SAFETY: kpathsea_bitmap_tolerance does not use its instance
        // argument.
        unsafe { kpathsea_bitmap_tolerance(std::ptr::null_mut(), dpi1, dpi2) != 0 }
    }
    #[cfg(not(feature = "kpathsea"))]
    {
        dpi1 == dpi2
    }
}

// ---------------------------------------------------------------------------
// pkin.c
// ---------------------------------------------------------------------------

/// `pkin.c`'s unpacking state (`inputbyte`, `flagbyte`, `bitweight`,
/// `dynf`, `repeatcount`, `realfunc`, `pk_remainder`), all C `halfword`s
/// (`int`) but `pk_remainder` (`long`).
#[derive(Default)]
struct Pk {
    inputbyte: i32,
    flagbyte: i32,
    bitweight: i32,
    dynf: i32,
    repeatcount: i32,
    /// `realfunc == rest` (else `pkpackednum`).
    real_is_rest: bool,
    remainder: i64,
}

/// `gpower`.
const GPOWER: [i32; 17] = [
    0, 1, 3, 7, 15, 31, 63, 127, 255, 511, 1023, 2047, 4095, 8191, 16383, 32767, 65535,
];

impl Globals {
    /// `pkbyte`.
    fn pkbyte(&mut self, f: &mut T3File) -> i32 {
        let i = f.getc();
        if i == -1 {
            self.pdftex_fail("unexpected eof in pk file");
        }
        i
    }

    /// `pkduo`, `pktrio`, `pkquad`: a signed big-endian number of 2, 3 or
    /// 4 bytes.
    fn pkmulti(&mut self, f: &mut T3File, n: usize) -> i32 {
        let mut i = self.pkbyte(f);
        if i > 127 {
            i -= 256;
        }
        for _ in 1..n {
            i = i.wrapping_mul(256).wrapping_add(self.pkbyte(f));
        }
        i
    }

    /// `getnyb`.
    fn getnyb(&mut self, pk: &mut Pk, f: &mut T3File) -> i32 {
        if pk.bitweight == 0 {
            pk.bitweight = 16;
            pk.inputbyte = self.pkbyte(f);
            pk.inputbyte >> 4
        } else {
            pk.bitweight = 0;
            pk.inputbyte & 15
        }
    }

    /// `getbit`.
    fn getbit(&mut self, pk: &mut Pk, f: &mut T3File) -> bool {
        pk.bitweight >>= 1;
        if pk.bitweight == 0 {
            pk.inputbyte = self.pkbyte(f);
            pk.bitweight = 128;
        }
        pk.inputbyte & pk.bitweight != 0
    }

    /// `(*realfunc)()`.
    fn realfunc(&mut self, pk: &mut Pk, f: &mut T3File) -> i32 {
        if pk.real_is_rest {
            self.pk_rest(pk)
        } else {
            self.pkpackednum(pk, f)
        }
    }

    /// `pkpackednum`.
    fn pkpackednum(&mut self, pk: &mut Pk, f: &mut T3File) -> i32 {
        let mut i = self.getnyb(pk, f);
        if i == 0 {
            let mut j;
            loop {
                j = self.getnyb(pk, f);
                i += 1;
                if j != 0 {
                    break;
                }
            }
            if i > 3 {
                // Damn, we got a huge count! We *fake* it by giving an
                // artificially large repeat count.
                self.handlehuge(pk, f, i, j)
            } else {
                while i > 0 {
                    j = j * 16 + self.getnyb(pk, f);
                    i -= 1;
                }
                j - 15 + (13 - pk.dynf) * 16 + pk.dynf
            }
        } else if i <= pk.dynf {
            i
        } else if i < 14 {
            (i - pk.dynf - 1) * 16 + self.getnyb(pk, f) + pk.dynf + 1
        } else {
            if i == 14 {
                pk.repeatcount = self.pkpackednum(pk, f);
            } else {
                pk.repeatcount = 1;
            }
            self.realfunc(pk, f)
        }
    }

    /// `rest`.
    fn pk_rest(&mut self, pk: &mut Pk) -> i32 {
        if pk.remainder < 0 {
            pk.remainder = -pk.remainder;
            0
        } else if pk.remainder > 0 {
            if pk.remainder > 4000 {
                pk.remainder = 4000 - pk.remainder;
                4000
            } else {
                let i = pk.remainder as i32;
                pk.remainder = 0;
                pk.real_is_rest = false;
                i
            }
        } else {
            self.pdftex_fail("shouldn't happen")
        }
    }

    /// `handlehuge`.
    fn handlehuge(&mut self, pk: &mut Pk, f: &mut T3File, mut i: i32, k: i32) -> i32 {
        let mut j = k as i64;
        while i != 0 {
            j = (j << 4) + self.getnyb(pk, f) as i64;
            i -= 1;
        }
        pk.remainder = j - 15 + (13 - pk.dynf as i64) * 16 + pk.dynf as i64;
        pk.real_is_rest = true;
        self.pk_rest(pk)
    }

    /// `unpack`: the raster of `cd` from the packed data that follows its
    /// preamble.
    fn unpack(&mut self, pk: &mut Pk, f: &mut T3File, cd: &mut CharDesc) {
        // shalfword wordwidth, rowsleft, hbit (C shorts)
        let wordwidth = ((cd.cwidth + 15) / 16) as i16;
        let mut size = 2i64 * cd.cheight as i64 * wordwidth as i64;
        if size <= 0 {
            size = 2;
        }
        cd.raster.clear();
        cd.raster.resize(size.max(0) as usize, 0);
        let mut r = 0usize; // raster pointer
        let put = |raster: &mut Vec<i32>, r: &mut usize, v: i32| {
            if *r >= raster.len() {
                raster.resize(*r + 1, 0);
            }
            raster[*r] = v;
            *r += 1;
        };
        pk.real_is_rest = false;
        pk.dynf = pk.flagbyte / 16;
        let mut turnon = pk.flagbyte & 8 != 0;
        if pk.dynf == 14 {
            pk.bitweight = 0;
            for _ in 1..=cd.cheight {
                let mut word = 0i32;
                let mut wordweight = 32768i32;
                for _ in 1..=cd.cwidth {
                    if self.getbit(pk, f) {
                        word += wordweight;
                    }
                    wordweight >>= 1;
                    if wordweight == 0 {
                        put(&mut cd.raster, &mut r, word);
                        word = 0;
                        wordweight = 32768;
                    }
                }
                if wordweight != 32768 {
                    put(&mut cd.raster, &mut r, word);
                }
            }
        } else {
            let mut rowsleft = cd.cheight as i16;
            let mut hbit = cd.cwidth as i16;
            pk.repeatcount = 0;
            let mut wordweight = 16i32;
            let mut word = 0i32;
            pk.bitweight = 0;
            while rowsleft > 0 {
                let mut count = self.realfunc(pk, f);
                while count != 0 {
                    if count < wordweight && count < hbit as i32 {
                        if turnon {
                            word += GPOWER[wordweight as usize]
                                - GPOWER[(wordweight - count) as usize];
                        }
                        hbit = (hbit as i32 - count) as i16;
                        wordweight -= count;
                        count = 0;
                    } else if count >= hbit as i32 && hbit as i32 <= wordweight {
                        if turnon {
                            word += GPOWER[wordweight as usize]
                                - GPOWER[(wordweight - hbit as i32) as usize];
                        }
                        put(&mut cd.raster, &mut r, word);
                        for _ in 1..=pk.repeatcount {
                            for _ in 1..=wordwidth {
                                let v = r
                                    .checked_sub(wordwidth as usize)
                                    .map_or(0, |q| cd.raster[q]);
                                put(&mut cd.raster, &mut r, v);
                            }
                        }
                        rowsleft = (rowsleft as i32 - (pk.repeatcount + 1)) as i16;
                        pk.repeatcount = 0;
                        word = 0;
                        wordweight = 16;
                        count -= hbit as i32;
                        hbit = cd.cwidth as i16;
                    } else {
                        if turnon {
                            word += GPOWER[wordweight as usize];
                        }
                        put(&mut cd.raster, &mut r, word);
                        word = 0;
                        count -= wordweight;
                        hbit = (hbit as i32 - wordweight) as i16;
                        wordweight = 16;
                    }
                }
                turnon = !turnon;
            }
            if rowsleft != 0 || hbit as i32 != cd.cwidth {
                self.pdftex_fail("error while unpacking; more bits than required");
            }
        }
    }

    /// `readchar`: check the preamble if asked, then read the next
    /// character definition into `cd`; false at the postamble.
    fn readchar(
        &mut self,
        pk: &mut Pk,
        f: &mut T3File,
        check_preamble: bool,
        cd: &mut CharDesc,
    ) -> bool {
        if check_preamble {
            if self.pkbyte(f) != 247 {
                self.pdftex_fail("bad pk file, expected pre");
            }
            if self.pkbyte(f) != 89 {
                self.pdftex_fail("bad version of pk file");
            }
            let mut i = self.pkbyte(f); // creator of pkfile
            while i > 0 {
                self.pkbyte(f);
                i -= 1;
            }
            self.pkmulti(f, 4); // design size
            self.pkmulti(f, 4); // checksum
            self.pkmulti(f, 4); // hppp
            self.pkmulti(f, 4); // vppp
        }
        loop {
            pk.flagbyte = self.pkbyte(f);
            if pk.flagbyte == 245 {
                return false;
            }
            if pk.flagbyte < 240 {
                let length: i64;
                match pk.flagbyte & 7 {
                    0..=3 => {
                        length = ((pk.flagbyte & 7) * 256 + self.pkbyte(f) - 3) as i64;
                        cd.charcode = self.pkbyte(f);
                        self.pkmulti(f, 3); // TFMwidth
                        cd.xescape = self.pkbyte(f); // pixel width
                        cd.cwidth = self.pkbyte(f);
                        cd.cheight = self.pkbyte(f);
                        cd.xoff = self.pkbyte(f);
                        cd.yoff = self.pkbyte(f);
                        if cd.xoff > 127 {
                            cd.xoff -= 256;
                        }
                        if cd.yoff > 127 {
                            cd.yoff -= 256;
                        }
                    }
                    4..=6 => {
                        let mut l = (pk.flagbyte & 3) as i64 * 65536 + self.pkbyte(f) as i64 * 256;
                        l = l + self.pkbyte(f) as i64 - 4;
                        length = l;
                        cd.charcode = self.pkbyte(f);
                        self.pkmulti(f, 3); // TFMwidth
                        cd.xescape = self.pkmulti(f, 2); // pixelwidth
                        cd.cwidth = self.pkmulti(f, 2);
                        cd.cheight = self.pkmulti(f, 2);
                        cd.xoff = self.pkmulti(f, 2);
                        cd.yoff = self.pkmulti(f, 2);
                    }
                    _ => {
                        length = self.pkmulti(f, 4) as i64 - 9;
                        cd.charcode = self.pkmulti(f, 4);
                        self.pkmulti(f, 4); // TFMwidth
                        cd.xescape = self.pkmulti(f, 4); // pixelwidth
                        self.pkmulti(f, 4);
                        cd.cwidth = self.pkmulti(f, 4);
                        cd.cheight = self.pkmulti(f, 4);
                        cd.xoff = self.pkmulti(f, 4);
                        cd.yoff = self.pkmulti(f, 4);
                    }
                }
                if length <= 0 {
                    self.pdftex_fail(&format!("packet length ({}) too small", length as i32));
                }
                self.unpack(pk, f, cd);
                return true;
            } else {
                let mut k: i32 = 0;
                match pk.flagbyte {
                    240..=243 => {
                        // the cases fall through: 243 reads 4 bytes, 240 one
                        let n = pk.flagbyte - 239;
                        for m in 0..n {
                            let b = self.pkbyte(f);
                            if m == 0 && pk.flagbyte == 243 {
                                k = if b > 127 { b - 256 } else { b };
                            } else {
                                k = k.wrapping_mul(256).wrapping_add(b);
                            }
                        }
                        while k > 0 {
                            k -= 1;
                            self.pkbyte(f);
                        }
                    }
                    244 => {
                        self.pkmulti(f, 4);
                    }
                    246 => {}
                    _ => self.pdftex_fail(&format!("unexpected command ({})", pk.flagbyte)),
                }
            }
        }
    }
}
