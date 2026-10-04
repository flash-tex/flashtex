//! Reading a Type 1 font a line at a time, as writet1.c's `t1_getline`
//! does: PFA (hexadecimal `eexec` part) or PFB (segments), the `eexec` part
//! decrypted, blanks and line ends normalised (`append_char_to_buf`), and
//! a charstring's binary bytes kept whole on the line that introduces it.

use super::cipher::Cipher;
use super::{Fail, Result};

/// `currentfile eexec`: where the encrypted part starts.
const EEXEC: &[u8] = b"currentfile eexec";

/// Where the reader is with respect to the encrypted part (`t1_in_eexec`:
/// 0, 1, 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Eexec {
    Before,
    Inside,
    After,
}

/// How the font file is packed (`t1_pfa`).
#[derive(Clone, Copy, Debug)]
enum Packing {
    /// PFA: the `eexec` part in hexadecimal.
    Pfa,
    /// PFB: segments, each `0x80 type length`; the bytes left in this one.
    Pfb { left: i64 },
}

/// One line of the font, as `t1_getline` leaves `t1_line_array`.
///
/// C keeps it as a NUL-terminated string that may also hold a charstring's
/// binary bytes, and reads it with string functions in some places and up
/// to `t1_line_ptr` in others; [`Line::text`] is the first view, `bytes`
/// the second.
#[derive(Default)]
pub(super) struct Line {
    pub bytes: Vec<u8>,
    /// The charstring on this line: its length (`t1_cslen`; 0 for none,
    /// also for an empty one) and where its bytes start.
    pub cs_len: u16,
    pub cs_start: usize,
}

impl Line {
    /// The line as C's string functions see it: up to its first NUL.
    pub fn text(&self) -> &[u8] {
        c_str(&self.bytes)
    }

    /// `t1_prefix(s)`.
    pub fn starts_with(&self, s: &[u8]) -> bool {
        self.text().starts_with(s)
    }

    /// `t1_suffix(s)`: the line ends with `s`, before its final LF.
    #[inline]
    pub fn ends_with(&self, s: &[u8]) -> bool {
        ends_with_before_lf(&self.bytes, s)
    }

    /// `strstr(t1_line_array, s) != NULL`.
    pub fn contains(&self, s: &[u8]) -> bool {
        find(self.text(), s).is_some()
    }

    /// The line for an error message (`remove_eol`).
    pub fn quoted(&self) -> String {
        without_eol(&self.bytes)
    }

    pub fn has_charstring(&self) -> bool {
        self.cs_len != 0
    }

    /// Replace the text, ending it with LF as `t1_line_ptr = eol(...)` does.
    pub fn set_text(&mut self, text: Vec<u8>) {
        self.bytes = text;
        end_line(&mut self.bytes);
    }
}

/// `s` up to its first NUL.
#[inline]
pub(super) fn c_str(s: &[u8]) -> &[u8] {
    s.iter().position(|&c| c == 0).map_or(s, |i| &s[..i])
}

/// `strstr`.
pub(super) fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// `str_suffix(begin, end, s)`: the bytes end with `s`, before a final LF.
#[inline]
pub(super) fn ends_with_before_lf(buf: &[u8], s: &[u8]) -> bool {
    buf.strip_suffix(b"\n").unwrap_or(buf).ends_with(s)
}

/// `eol(s)`: cut the C string at its NUL and end it with LF, unless it is
/// shorter than two bytes or ends with LF already.
pub(super) fn end_line(s: &mut Vec<u8>) {
    let n = c_str(s).len();
    s.truncate(n);
    if n > 1 && s[n - 1] != b'\n' {
        s.push(b'\n');
    }
}

/// `remove_eol`: a line for an error message, without its LF.
pub(super) fn without_eol(s: &[u8]) -> String {
    let s = c_str(s);
    String::from_utf8_lossy(s.strip_suffix(b"\n").unwrap_or(s)).into_owned()
}

/// `append_char_to_buf` (ptexmac.h): tab to blank, CR and end of file to
/// LF, no leading or doubled blanks. Returns the character as converted.
#[inline]
pub(super) fn append_char(c: Option<u8>, buf: &mut Vec<u8>) -> u8 {
    let c = match c {
        Some(b'\t') => b' ',
        Some(b'\r') | None => b'\n',
        Some(c) => c,
    };
    if c != b' ' || buf.last().is_some_and(|&l| l != b' ') {
        buf.push(c);
    }
    c
}

/// `append_eol` (ptexmac.h): end the line with one LF, dropping a blank
/// before it.
pub(super) fn append_eol(buf: &mut Vec<u8>) {
    if buf.len() > 1 && buf.last() != Some(&b'\n') {
        buf.push(b'\n');
    }
    if buf.len() > 2 && buf[buf.len() - 2] == b' ' {
        buf.pop();
        if let Some(last) = buf.last_mut() {
            *last = b'\n';
        }
    }
}

/// `t1_scan_num(p, &r)`: the number at `line[p..]` after one optional
/// blank (read by C's `sscanf("%g")`), and where its characters end.
pub(super) fn scan_num(line: &[u8], p: usize) -> Result<(f32, usize)> {
    let p = if line.get(p) == Some(&b' ') { p + 1 } else { p };
    let rest = c_str(line.get(p..).unwrap_or_default());
    let Some(value) = crate::pdftex::cfmt::scan_float(rest) else {
        return Err(Fail(format!("a number expected: `{}'", without_eol(line))));
    };
    let len = line[p.min(line.len())..]
        .iter()
        .take_while(|&&c| c.is_ascii_digit() || matches!(c, b'.' | b'e' | b'E' | b'+' | b'-'))
        .count();
    Ok((value, p + len))
}

/// The one error reading a byte can give: a PFB segment that does not
/// start with its marker. A type of no size, so that the per-byte reads
/// return in registers.
pub(super) struct BadMarker;

impl From<BadMarker> for Fail {
    fn from(_: BadMarker) -> Fail {
        Fail("invalid marker".into())
    }
}

/// A font file being read (`t1_file` and the reading state of writet1.c).
pub(super) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    /// `feof(t1_file)`: a read has gone past the end.
    eof: bool,
    packing: Packing,
    /// The `eexec` decryption key (`t1_dr`).
    key: Cipher,
    /// The last byte decoded from hexadecimal (`t1_lastchar`'s use in
    /// `t1_stop_eexec`).
    last_hexbyte: u8,
    pub eexec: Eexec,
    pub line: Line,
}

impl<'a> Reader<'a> {
    /// `t1_init_params`' part for reading (`t1_check_pfa`: a PFB starts
    /// with its first segment marker).
    pub fn new(data: &'a [u8]) -> Self {
        Reader {
            data,
            pos: 0,
            eof: false,
            packing: if data.first() == Some(&0x80) {
                Packing::Pfb { left: 0 }
            } else {
                Packing::Pfa
            },
            key: Cipher::EEXEC,
            last_hexbyte: 0,
            eexec: Eexec::Before,
            line: Line::default(),
        }
    }

    #[inline]
    pub fn is_pfa(&self) -> bool {
        matches!(self.packing, Packing::Pfa)
    }

    /// `t1_getchar()`: `getc`.
    #[inline]
    fn next_raw(&mut self) -> Option<u8> {
        let b = self.data.get(self.pos).copied();
        match b {
            Some(_) => self.pos += 1,
            None => self.eof = true,
        }
        b
    }

    /// `t1_getbyte`: the next data byte, across PFB segment headers; `None`
    /// at the end of the file or at its end-of-file segment.
    #[inline]
    fn next_byte(&mut self) -> Result<Option<u8>, BadMarker> {
        let c = self.next_raw();
        let Packing::Pfb { left } = self.packing else {
            return Ok(c);
        };
        let (c, left) = if left > 0 {
            (c, left)
        } else {
            if c != Some(0x80) {
                return Err(BadMarker);
            }
            if self.next_raw() == Some(3) {
                self.pos = self.data.len();
                self.eof = true;
                return Ok(None);
            }
            // C reads the four length bytes with `getc() & 0xff`: past the
            // end of the file each is 0xff.
            let mut len = [0u8; 4];
            for b in &mut len {
                *b = self.next_raw().unwrap_or(0xff);
            }
            (self.next_raw(), i64::from(u32::from_le_bytes(len)))
        };
        self.packing = Packing::Pfb { left: left - 1 };
        Ok(c)
    }

    /// The bytes left in the PFB segment being read (0 in a PFA).
    #[inline]
    fn segment_left(&self) -> i64 {
        match self.packing {
            Packing::Pfa => 0,
            Packing::Pfb { left } => left,
        }
    }

    /// `edecrypt`: decrypt one byte of the `eexec` part; in a PFA, `cipher`
    /// is the first hexadecimal digit of the byte, after any line ends.
    #[inline]
    fn decrypt(&mut self, cipher: Option<u8>) -> Result<u8, BadMarker> {
        // C passes `(byte) t1_getbyte()`: the end of the file is 0xff.
        let mut cipher = cipher.unwrap_or(0xff);
        if self.is_pfa() {
            while cipher == b'\n' || cipher == b'\r' {
                cipher = self.next_byte()?.unwrap_or(0xff);
            }
            let hi = hex_value(Some(cipher));
            let lo = hex_value(self.next_byte()?);
            cipher = ((hi << 4) + lo) as u8;
            self.last_hexbyte = cipher;
        }
        Ok(self.key.decrypt(cipher))
    }

    /// `t1_getline`: the next non-empty line into [`Reader::line`]. With
    /// `charstrings` (`t1_cs`), a line that reaches ` RD ` or ` -| ` takes
    /// the charstring bytes its number announces. At the end of the file
    /// the line is left empty; a further read is an error.
    pub fn read_line(&mut self, charstrings: bool) -> Result<()> {
        loop {
            if self.eof {
                return Err(Fail("unexpected end of file".into()));
            }
            let line = &mut self.line;
            line.bytes.clear();
            line.cs_len = 0;
            // how much of `currentfile eexec` the line has matched so far
            let mut eexec_match = Some(0usize);
            let Some(mut c) = self.next_byte()? else {
                return Ok(());
            };
            while !self.eof {
                if self.eexec == Eexec::Inside {
                    c = self.decrypt(Some(c))?;
                }
                let c_out = append_char(Some(c), &mut self.line.bytes);
                if self.eexec == Eexec::Before {
                    if let Some(k) = eexec_match.filter(|&k| k < EEXEC.len()) {
                        eexec_match = (self.line.bytes.get(k) == Some(&EEXEC[k])).then_some(k + 1);
                    }
                }
                let eexec_found = eexec_match == Some(EEXEC.len());
                if c_out == b'\n' || (self.is_pfa() && eexec_found && c_out == b' ') {
                    break;
                }
                let line = &self.line;
                if charstrings
                    && line.cs_len == 0
                    && line.bytes.len() > 4
                    && (line.ends_with(b" RD ") || line.ends_with(b" -| "))
                {
                    self.read_charstring()?;
                }
                c = match self.next_byte()? {
                    Some(c) => c,
                    None => break, // `eof` is set
                };
            }
            append_eol(&mut self.line.bytes);
            if self.line.bytes.len() < 2 {
                continue;
            }
            if eexec_match == Some(EEXEC.len()) {
                self.eexec = Eexec::Inside;
            }
            return Ok(());
        }
    }

    /// The charstring after ` RD `: its length is the number before that,
    /// after the line's previous blank.
    fn read_charstring(&mut self) -> Result<()> {
        let bytes = &self.line.bytes;
        let before = &bytes[..bytes.len() - 4];
        // C searches back for the blank without a bound; none before the
        // number would be undefined behaviour there, here the number starts
        // the line.
        let start = before.iter().rposition(|&c| c == b' ').map_or(0, |p| p + 1);
        let len = scan_num(bytes, start)?.0 as i32;
        self.line.cs_len = len as u16;
        self.line.cs_start = self.line.bytes.len();
        for _ in 0..len.max(0) {
            let b = self.next_byte()?;
            let plain = self.decrypt(b)?;
            self.line.bytes.push(plain);
        }
        Ok(())
    }

    /// `t1_check_block_len`: a PFB segment must end where its header said,
    /// with a line end.
    fn check_segment_end(&mut self, decrypt: bool) -> Result<()> {
        if self.segment_left() == 0 {
            return Ok(());
        }
        let mut c = self.next_byte()?;
        if decrypt {
            c = Some(self.decrypt(c)?);
        }
        let left = self.segment_left();
        if !(left == 0 && matches!(c, Some(b'\n' | b'\r'))) {
            return Err(Fail(format!("{} bytes more than expected", left + 1)));
        }
        Ok(())
    }

    /// The start of `t1_start_eexec`: the PFB's clear segment must have
    /// ended, then the four random bytes that start the encrypted part are
    /// read (and dropped).
    pub fn start_eexec(&mut self) -> Result<()> {
        if !self.is_pfa() {
            self.check_segment_end(false)?;
        }
        for _ in 0..4 {
            let b = self.next_byte()?;
            self.decrypt(b)?;
        }
        Ok(())
    }

    /// The reading half of `t1_stop_eexec`: what follows the encrypted part.
    /// `Ok(true)` when a PFA's last hexadecimal byte was `00` and the next
    /// is not a line end: pdfTeX then writes `00` (its fix for fonts whose
    /// zeros run on).
    pub fn stop_eexec(&mut self) -> Result<bool> {
        let mut put_zeros = false;
        if !self.is_pfa() {
            self.check_segment_end(true)?;
        } else {
            let b = self.next_byte()?;
            let c = self.decrypt(b)?;
            if !(c == b'\n' || c == b'\r') {
                if self.last_hexbyte == 0 {
                    put_zeros = true;
                } else {
                    return Err(Fail("unexpected data after eexec".into()));
                }
            }
        }
        self.eexec = Eexec::After;
        Ok(put_zeros)
    }
}

/// A hexadecimal digit's value, or -1.
#[inline]
fn hex_value(c: Option<u8>) -> i32 {
    match c {
        Some(c @ b'A'..=b'F') => i32::from(c - b'A') + 10,
        Some(c @ b'a'..=b'f') => i32::from(c - b'a') + 10,
        Some(c @ b'0'..=b'9') => i32::from(c - b'0'),
        _ => -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_char_normalises_blanks_and_line_ends() {
        let mut buf = Vec::new();
        for c in b"  a\t\tb \r".iter().copied().map(Some).chain([None]) {
            append_char(c, &mut buf);
        }
        assert_eq!(buf, b"a b \n\n");
        let mut line = b"abc ".to_vec();
        append_eol(&mut line);
        assert_eq!(line, b"abc\n");
    }

    #[test]
    fn lines_of_a_pfa() {
        let font = b"%!PS\r\n/FontType 1 def\n\ncurrentfile eexec\n";
        let mut r = Reader::new(font);
        r.read_line(false).unwrap();
        assert_eq!(r.line.bytes, b"%!PS\n");
        r.read_line(false).unwrap();
        assert_eq!(r.line.bytes, b"/FontType 1 def\n");
        r.read_line(false).unwrap();
        assert_eq!(r.line.bytes, b"currentfile eexec\n");
        assert_eq!(r.eexec, Eexec::Inside);
    }

    #[test]
    fn a_pfb_segment_header_is_not_data() {
        let mut font = vec![0x80, 1, 6, 0, 0, 0];
        font.extend_from_slice(b"%!a\nb\n");
        font.extend_from_slice(&[0x80, 3]);
        let mut r = Reader::new(&font);
        assert!(!r.is_pfa());
        r.read_line(false).unwrap();
        assert_eq!(r.line.bytes, b"%!a\n");
        r.read_line(false).unwrap();
        assert_eq!(r.line.bytes, b"b\n");
        r.read_line(false).unwrap();
        assert!(r.line.bytes.is_empty());
        assert!(r.read_line(false).is_err());
    }

    #[test]
    fn scan_num_reads_like_sscanf() {
        assert_eq!(scan_num(b"/lenIV 4 def\n", 6).unwrap(), (4.0, 8));
        assert_eq!(scan_num(b"x -1.5e2]", 1).unwrap(), (-150.0, 8));
        assert!(scan_num(b"/x abc\n", 2).is_err());
    }
}
