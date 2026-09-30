//! The PDF byte writer: `pdftex.h`'s `writepdf`, `writezip.c`, and the
//! output routines of `utils.c` (the font buffer `fb_*`, `pdf_puts`,
//! `pdf_printf`, `writestreamlength`, `printID`, `printcreationdate`,
//! `removepdffile`, `make_subset_tag`), ported.
//!
//! Streams are compressed with TeX Live's own zlib (third_party/zlib, see
//! [`super::zlib`]), called exactly as `writezip.c` calls it, so the
//! compressed bytes are pdfTeX's.

use super::zlib::{ZStream, Z_FINISH, Z_NO_FLUSH, Z_OK, Z_STREAM_END};
use super::{md5, with_state};
use crate::generated::Globals;
use std::collections::BTreeSet;

/// `ZIP_BUF_SIZE` (writezip.c).
const ZIP_BUF_SIZE: usize = 32768;
/// `PRINTF_BUF_SIZE` (ptexmac.h): `pdf_printf` and `tex_printf` format into
/// a buffer of this size with `vsnprintf`, which cuts longer output.
pub const PRINTF_BUF_SIZE: usize = 1024;

/// The C globals of the output routines.
#[derive(Default)]
pub struct State {
    /// `fb_array` up to `fb_ptr`: the font file being built.
    pub fb: Vec<u8>,
    /// `c_stream` and `zipbuf` (writezip.c), once `writezip` has run.
    zip: Option<(Box<ZStream>, Vec<u8>)>,
    /// writezip.c's `level_old`.
    level_old: i32,
    /// `cur_file_name` (utils.c): named by `pdftex_warn` and `pdftex_fail`.
    pub cur_file_name: Option<Vec<u8>>,
    /// make_subset_tag's `st_tree`: the subset tags given out.
    subset_tags: BTreeSet<[u8; 6]>,
}

/// `cur_file_name = s` (utils.c's global).
pub fn set_cur_file_name(s: Option<&[u8]>) {
    with_state(|st| st.out.cur_file_name = s.map(|s| s.to_vec()));
}

/// The current `cur_file_name`.
pub fn cur_file_name() -> Option<Vec<u8>> {
    with_state(|st| st.out.cur_file_name.clone())
}

/// `vsnprintf(print_buf, PRINTF_BUF_SIZE, ...)`: at most 1023 bytes.
pub(super) fn printf_cut(s: &[u8]) -> &[u8] {
    &s[..s.len().min(PRINTF_BUF_SIZE - 1)]
}

impl Globals {
    // -----------------------------------------------------------------------
    // pdftex.h, writezip.c
    // -----------------------------------------------------------------------

    /// `writepdf(a, b)`: bytes `a..=b` of the PDF buffer to the PDF file.
    pub fn write_pdf(&mut self, a: i32, b: i32) {
        crate::displaylist::tap(self);
        let bytes = self.pdf_buf_bytes(a, b + 1);
        self.pdf_file.write_bytes(&bytes);
    }

    /// Bytes `a..b` of the PDF buffer (`pdf_buf_get` of each, a slice at a
    /// time: images pass megabytes through here).
    fn pdf_buf_bytes(&self, a: i32, b: i32) -> Vec<u8> {
        let buf = if self.pdf_buf_is_os {
            &self.pdf_os_buf
        } else {
            &self.pdf_op_buf
        };
        buf[a as usize..b as usize]
            .iter()
            .map(|&x| x as u8)
            .collect()
    }

    /// `writezip` (writezip.c): compress the PDF buffer into the file.
    pub fn write_zip(&mut self, finish: bool) {
        crate::displaylist::tap(self);
        let level = self.get_pdf_compress_level();
        self.pdfassert(level > 0);
        set_cur_file_name(None);
        let (mut zs, mut zipbuf, level_old) = match with_state(|s| s.out.zip.take()) {
            Some((z, b)) => (z, b, with_state(|s| s.out.level_old)),
            None => (ZStream::new(), Vec::new(), 0),
        };
        if self.pdf_stream_length == 0 {
            if zipbuf.is_empty() {
                zipbuf = vec![0u8; ZIP_BUF_SIZE];
                self.check_zip_err(zs.deflate_init(level), "deflateInit");
            } else if level != level_old {
                // \pdfcompresslevel changed in mid document
                self.check_zip_err(zs.deflate_end(), "deflateEnd");
                self.check_zip_err(zs.deflate_init(level), "deflateInit");
            } else {
                self.check_zip_err(zs.deflate_reset(), "deflateReset");
            }
            with_state(|s| s.out.level_old = level);
            zs.next_out = zipbuf.as_mut_ptr();
            zs.avail_out = ZIP_BUF_SIZE as u32;
        }
        // `c_stream.next_in = pdfbuf`: the PDF output buffer, as bytes.
        let input = self.pdf_buf_bytes(0, self.pdf_ptr);
        zs.next_in = input.as_ptr();
        zs.avail_in = input.len() as u32;
        loop {
            if zs.avail_out == 0 {
                self.pdf_gone += self.xfwrite(&zipbuf) as i64;
                self.pdf_last_byte = zipbuf[ZIP_BUF_SIZE - 1] as i32;
                zs.next_out = zipbuf.as_mut_ptr();
                zs.avail_out = ZIP_BUF_SIZE as u32;
            }
            let err = zs.deflate(if finish { Z_FINISH } else { Z_NO_FLUSH });
            if finish && err == Z_STREAM_END {
                break;
            }
            self.check_zip_err(err, "deflate");
            if !finish && zs.avail_in == 0 {
                break;
            }
        }
        // `input` is dropped below; zlib keeps no pointer into it once
        // `avail_in` is 0, which both exits above guarantee.
        zs.next_in = std::ptr::null();
        if finish {
            let used = ZIP_BUF_SIZE - zs.avail_out as usize;
            if used > 0 {
                self.pdf_gone += self.xfwrite(&zipbuf[..used]) as i64;
                self.pdf_last_byte = zipbuf[used - 1] as i32;
            }
            use crate::system::PasFile;
            self.pdf_file.flush();
        }
        self.pdf_stream_length = zs.total_out as i64;
        with_state(|s| s.out.zip = Some((zs, zipbuf)));
    }

    fn check_zip_err(&mut self, err: i32, f: &str) {
        if err != Z_OK {
            self.pdftex_fail(&format!("zlib: {f}() failed (error code {err})"));
        }
    }

    /// `xfwrite(ptr, 1, n, pdffile)` (utils.c).
    fn xfwrite(&mut self, bytes: &[u8]) -> usize {
        if !self.pdf_file.write_bytes(bytes) {
            self.pdftex_fail("fwrite() failed");
        }
        bytes.len()
    }

    /// `zip_free` (writezip.c).
    fn zip_free(&mut self) {
        if let Some((mut zs, _)) = with_state(|s| s.out.zip.take()) {
            self.check_zip_err(zs.deflate_end(), "deflateEnd");
        }
    }

    /// `getc(f)` on a binary file: the next byte, or -1 at the end.
    pub fn getc(&mut self, f: &mut crate::system::ByteFile) -> i32 {
        crate::system::getc(f)
    }

    // -----------------------------------------------------------------------
    // utils.c: output
    // -----------------------------------------------------------------------

    /// `writestreamlength` (utils.c): patch the `/Length` of the stream just
    /// written, then return to the end of the file.
    pub fn write_stream_length(&mut self, length: i64, offset: i64) {
        if self.fixed_pdf_draftmode == 0 {
            let end = (self.pdf_gone + self.pdf_ptr as i64) as u64;
            let ok = self.pdf_file.seek_to(offset as u64)
                && self.pdf_file.write_bytes(length.to_string().as_bytes())
                && self.pdf_file.seek_to(end);
            if !ok {
                let job = self.str_bytes(self.job_name);
                self.pdftex_fail(&format!(
                    "fseeko() failed on {}",
                    String::from_utf8_lossy(&job)
                ));
            }
        }
    }

    /// `removepdffile` (utils.c): delete the unfinished PDF file.
    pub fn remove_pdffile(&mut self) {
        if self.output_file_name != 0 && self.fixed_pdf_draftmode == 0 {
            let mut f = std::mem::take(&mut self.pdf_file);
            self.b_close(&mut f);
            self.pdf_file = f;
            let name = self.str_bytes(self.output_file_name);
            let _ = std::fs::remove_file(String::from_utf8_lossy(&name).as_ref());
        }
    }

    /// `libpdffinish` (utils.c): free the C parts' memory. Only zlib's
    /// stream holds anything that is not Rust's to drop.
    pub fn libpdffinish(&mut self) {
        with_state(|s| s.out.fb = Vec::new());
        self.zip_free();
    }

    /// `pdfroom` (ptexmac.h): the C parts' version of pdftex.web's
    /// `pdf_room`.
    pub fn c_pdf_room(&mut self, n: i32) {
        if (n + self.pdf_ptr) as u32 > self.pdf_buf_size as u32 {
            if self.pdf_os_mode {
                self.pdf_os_get_os_buf(n);
            } else if n as u32 > self.pdf_buf_size as u32 {
                self.pdftex_fail("PDF output buffer overflowed");
            } else {
                self.pdf_flush();
            }
        }
    }

    /// `pdf_puts` (utils.c).
    pub fn pdf_puts(&mut self, s: &[u8]) {
        self.c_pdf_room(s.len() as i32 + 1);
        for &b in s {
            let p = self.pdf_ptr;
            self.pdf_buf_set(p, b as i32);
            self.pdf_ptr += 1;
        }
        // `pdflastbyte = s[-1]`: for an empty string the byte before it,
        // which no caller passes.
        if let Some(&b) = s.last() {
            self.pdf_last_byte = b as i32;
        }
    }

    /// `pdf_printf` (utils.c): `pdf_puts` of the formatted text, cut as
    /// `vsnprintf` cuts it.
    pub fn pdf_printf(&mut self, s: &[u8]) {
        self.pdf_puts(printf_cut(s));
    }

    /// `pdf_newline` (utils.c).
    pub fn pdf_newline(&mut self) {
        if self.pdf_last_byte != b'\n' as i32 {
            self.pdf_puts(b"\n");
        }
    }

    /// `tex_printf` (utils.c): print the formatted text on the terminal
    /// and in the log (`print` of a new string prints its characters with
    /// `print_char`).
    pub fn tex_printf(&mut self, s: &[u8]) {
        let s = printf_cut(s).to_vec();
        self.print_bytes(&s);
    }

    /// `fb_offset` (utils.c).
    pub fn fb_offset(&self) -> i32 {
        with_state(|s| s.out.fb.len() as i32)
    }

    /// `fb_putchar` (utils.c).
    pub fn fb_putchar(&mut self, b: u8) {
        with_state(|s| s.out.fb.push(b));
    }

    /// `fb_flush` (utils.c): the font buffer into the PDF buffer.
    pub fn fb_flush(&mut self) {
        let fb = with_state(|s| std::mem::take(&mut s.out.fb));
        let mut p = 0usize;
        while p < fb.len() {
            let mut n = (self.pdf_buf_size - self.pdf_ptr) as usize;
            if fb.len() - p < n {
                n = fb.len() - p;
            }
            for &b in &fb[p..p + n] {
                let q = self.pdf_ptr;
                self.pdf_buf_set(q, b as i32);
                self.pdf_ptr += 1;
            }
            if self.pdf_ptr == self.pdf_buf_size {
                self.pdf_flush();
            }
            p += n;
        }
        with_state(|s| {
            let mut fb = fb;
            fb.clear();
            s.out.fb = fb;
        });
    }

    /// `make_subset_tag` (utils.c): six letters from the MD5 of the glyph
    /// names and the font name, unique among the tags given so far.
    pub fn make_subset_tag<'a>(
        &mut self,
        glyphs: impl Iterator<Item = &'a Vec<u8>> + Clone,
        fontname: &[u8],
    ) -> [u8; 6] {
        let mut j: i32 = 0;
        let tag = loop {
            let mut data = Vec::new();
            for g in glyphs.clone() {
                data.extend_from_slice(g);
                data.push(b' ');
            }
            data.extend_from_slice(fontname);
            // `md5_append(&pms, &j, sizeof(int))`: the int's bytes in memory.
            data.extend_from_slice(&j.to_ne_bytes());
            let digest = md5::digest(&data);
            let mut a = [0i32; 6];
            for &d in &digest[..13] {
                a[0] += d as i32;
            }
            for i in 1..6 {
                a[i] = a[i - 1] - digest[i - 1] as i32 + digest[(i + 12) % 16] as i32;
            }
            let mut tag = [0u8; 6];
            for i in 0..6 {
                tag[i] = (a[i] % 26) as u8 + b'A';
            }
            j += 1;
            self.pdfassert(j < 100);
            if !with_state(|s| s.out.subset_tags.contains(&tag)) {
                break tag;
            }
        };
        with_state(|s| s.out.subset_tags.insert(tag));
        if j > 2 {
            self.pdftex_warn(&format!(
                "\nmake_subset_tag(): subset-tag collision, resolved in round {j}.\n"
            ));
        }
        tag
    }

    /// `printID` (utils.c): `/ID` from the MD5 of the start time and the
    /// output file name.
    #[allow(non_snake_case)] // pdftex.web's name
    pub fn print_ID(&mut self, filename: i32) {
        let mut data = super::utils::start_time_str();
        data.extend_from_slice(&self.str_bytes(filename));
        let id = hex_upper(&md5::digest(&data));
        self.pdf_printf(format!("/ID [<{id}> <{id}>]").as_bytes());
    }

    /// `printIDalt` (utils.c): `/ID` from the MD5 of `\pdftrailerid`.
    #[allow(non_snake_case)] // pdftex.web's name
    pub fn print_ID_alt(&mut self, toks: i32) {
        let s = self.tokens_to_string(toks);
        let bytes = self.str_bytes(s);
        self.flush_str(self.last_tokens_string);
        // makecstring stops at the first NUL.
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        if end == 0 {
            return;
        }
        let id = hex_upper(&md5::digest(&bytes[..end]));
        self.pdf_printf(format!("/ID [<{id}> <{id}>]").as_bytes());
    }

    /// `printcreationdate` (utils.c).
    pub fn print_creation_date(&mut self) {
        let mut s = b"/CreationDate (".to_vec();
        s.extend_from_slice(&super::utils::start_time_str());
        s.extend_from_slice(b")\n");
        self.pdf_printf(&s);
    }

    /// `printmoddate` (utils.c).
    pub fn print_mod_date(&mut self) {
        let mut s = b"/ModDate (".to_vec();
        s.extend_from_slice(&super::utils::start_time_str());
        s.extend_from_slice(b")\n");
        self.pdf_printf(&s);
    }
}

/// texmfmp.c's `convertStringToHexString`: `%02X` per byte.
fn hex_upper(d: &[u8]) -> String {
    d.iter().map(|b| format!("{b:02X}")).collect()
}
