//! Type 1 font programs: bounded parsing and glyph subsetting for embedding.
//!
//! pdfTeX embeds Type 1 fonts as subsets: only the charstrings the document
//! uses, unused subroutines blanked, the private dictionary re-encrypted.
//! This module does the same so a Type 1 embedding by this crate can be
//! compared with pdfTeX's apples to apples (`crate::compare` reports whether
//! the decrypted charstrings of the glyphs both sides embed are identical).
//!
//! What is read (Adobe Type 1 Font Format, chapters 2, 6, 7 and 8; the
//! Commander's passive reader in `crates/font-resources` documents the same
//! conventions and was consulted, not copied):
//!
//! - PFB segments (`0x80 0x01` ASCII, `0x80 0x02` binary, `0x80 0x03` end),
//!   or the raw `Length1/2/3` form a PDF `FontFile` carries.
//! - eexec decryption of the binary section (r = 55665, four leading
//!   bytes discarded) and charstring decryption (r = 4330, `lenIV` bytes,
//!   default 4).
//! - `/Subrs n array` with `dup i len RD … NP` entries and `/CharStrings n
//!   dict dup begin` with `/name len RD … ND` entries, where the reader
//!   procedure name is whatever the font defines (`RD` or `-|`), the length
//!   is a literal integer and exactly one space precedes the binary bytes.
//! - Type 1 charstring operators, enough to find `callsubr` (subroutine
//!   index = last operand), `callothersubr` 3 hint replacement (the
//!   subroutine number it pushes back with `pop`), and `seac` (base and
//!   accent glyphs by StandardEncoding code).
//!
//! What is written by [`Type1Font::subset`]: the clear-text portion
//! verbatim (`Length1` unchanged, so `/Encoding`, `/FontName`, `/FontBBox`
//! stay the source's), the private dictionary with every retained subroutine
//! byte-for-byte, every unused subroutine replaced by an encrypted `return`
//! (indices unchanged, so no charstring is rewritten), `/CharStrings` holding
//! only the retained glyphs with their original encrypted bytes, everything
//! else in the dictionary verbatim, re-encrypted with four zero prefix bytes
//! (deterministic), and no trailer (`Length3 = 0`, as pdfTeX writes).
//! Retained charstrings therefore decrypt to the source bytes exactly.
//!
//! Bounded and explicit: PFA (hex eexec) input, `lenIV` outside `0..=32`,
//! unexpected dictionary layouts, more than 65536 subroutines or glyphs, or
//! a glyph absent from the font are errors, never guesses.

use crate::exact::FontProgram;
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_PROGRAM_BYTES: usize = 64 * 1024 * 1024;
const EEXEC_R: u16 = 55665;
const CHARSTRING_R: u16 = 4330;
const C1: u16 = 52845;
const C2: u16 = 22719;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type1Error {
    Truncated(&'static str),
    Malformed(String),
    Unsupported(String),
    MissingGlyph(String),
    Limit(&'static str),
}

impl std::fmt::Display for Type1Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type1Error::Truncated(w) => write!(f, "Type 1 program truncated at {w}"),
            Type1Error::Malformed(m) => write!(f, "malformed Type 1 program: {m}"),
            Type1Error::Unsupported(m) => write!(f, "unsupported Type 1 program: {m}"),
            Type1Error::MissingGlyph(n) => write!(f, "glyph /{n} is not in the font"),
            Type1Error::Limit(w) => write!(f, "Type 1 limit exceeded: {w}"),
        }
    }
}

type R<T> = Result<T, Type1Error>;

/// The eexec/charstring byte recurrence (chapter 7). `skip` leading
/// plaintext bytes are discarded.
pub fn decrypt(cipher: &[u8], mut r: u16, skip: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(cipher.len());
    for &c in cipher {
        out.push(c ^ (r >> 8) as u8);
        r = (c as u16).wrapping_add(r).wrapping_mul(C1).wrapping_add(C2);
    }
    out.drain(..skip.min(out.len()));
    out
}

/// The inverse recurrence with `prefix` plaintext bytes prepended.
pub fn encrypt(plain: &[u8], mut r: u16, prefix: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(plain.len() + prefix.len());
    for &p in prefix.iter().chain(plain) {
        let c = p ^ (r >> 8) as u8;
        out.push(c);
        r = (c as u16).wrapping_add(r).wrapping_mul(C1).wrapping_add(C2);
    }
    out
}

/// One binary record inside the decrypted private dictionary: the byte
/// range of its encrypted charstring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Record {
    start: usize,
    end: usize,
}

/// A parsed Type 1 program.
#[derive(Debug, Clone)]
pub struct Type1Font {
    clear: Vec<u8>,
    /// Decrypted private portion (the four eexec prefix bytes removed).
    plain: Vec<u8>,
    trailer: Vec<u8>,
    len_iv: usize,
    /// `dup i len RD` entries by index, with the range of the whole entry
    /// line (`dup` … `NP`) and of its binary bytes.
    subrs: Vec<Option<(Record, Record)>>,
    subrs_header: Record,
    /// Byte range of the `/Subrs … NP` entries block (first `dup` to the
    /// end of the last entry's terminator).
    subrs_block: Record,
    /// Glyph name to (entry range, binary range), in file order.
    glyphs: Vec<(String, Record, Record)>,
    charstrings_header: Record,
    /// Byte range of the `/CharStrings n dict dup begin` header line.
    charstrings_block: Record,
    reader: Vec<u8>,
    subr_terminator: Vec<u8>,
    glyph_terminator: Vec<u8>,
    font_name: Option<String>,
    font_bbox: Option<[i32; 4]>,
    italic_angle: Option<String>,
    std_vw: Option<i32>,
}

fn find(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if from > hay.len() {
        return None;
    }
    hay[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\n' | b'\r' | b'\t' | b'\x0c' | b'\0')
}

/// Reads one whitespace-delimited token starting at or after `at`.
fn token(b: &[u8], at: usize) -> Option<(usize, usize)> {
    let mut i = at;
    while i < b.len() && is_ws(b[i]) {
        i += 1;
    }
    if i >= b.len() {
        return None;
    }
    let start = i;
    while i < b.len() && !is_ws(b[i]) {
        i += 1;
    }
    Some((start, i))
}

fn int_token(b: &[u8], at: usize, what: &str) -> R<(i64, usize)> {
    let (s, e) = token(b, at).ok_or_else(|| Type1Error::Malformed(format!("{what}: missing")))?;
    let v = std::str::from_utf8(&b[s..e])
        .ok()
        .and_then(|t| t.parse::<i64>().ok())
        .ok_or_else(|| {
            Type1Error::Malformed(format!(
                "{what}: {:?} is not an integer",
                String::from_utf8_lossy(&b[s..e])
            ))
        })?;
    Ok((v, e))
}

impl Type1Font {
    /// Parses a PFB file.
    pub fn parse_pfb(pfb: &[u8]) -> R<Type1Font> {
        match FontProgram::type1_from_pfb(pfb).map_err(Type1Error::Malformed)? {
            FontProgram::Type1 {
                bytes,
                length1,
                length2,
                length3,
            } => Self::parse_program(&bytes, length1, length2, length3),
            _ => unreachable!(),
        }
    }

    /// Parses the `Length1/2/3` form (clear text, binary eexec, trailer).
    pub fn parse_program(
        bytes: &[u8],
        length1: usize,
        length2: usize,
        length3: usize,
    ) -> R<Type1Font> {
        if bytes.len() > MAX_PROGRAM_BYTES {
            return Err(Type1Error::Limit("program larger than 64 MiB"));
        }
        if length1 + length2 + length3 != bytes.len() {
            return Err(Type1Error::Malformed(format!(
                "Length1+2+3 = {} but the program has {} bytes",
                length1 + length2 + length3,
                bytes.len()
            )));
        }
        let clear = bytes[..length1].to_vec();
        let binary = &bytes[length1..length1 + length2];
        let trailer = bytes[length1 + length2..].to_vec();
        if !clear.starts_with(b"%!") {
            return Err(Type1Error::Malformed(
                "clear text does not start with %!".into(),
            ));
        }
        if binary.len() < 4 {
            return Err(Type1Error::Truncated("eexec section"));
        }
        if binary[..4].iter().all(|b| b.is_ascii_hexdigit()) {
            return Err(Type1Error::Unsupported(
                "hex (PFA) eexec section; binary eexec only".into(),
            ));
        }
        let plain = decrypt(binary, EEXEC_R, 4);
        let mut font = Type1Font {
            clear,
            plain,
            trailer,
            len_iv: 4,
            subrs: Vec::new(),
            subrs_header: Record { start: 0, end: 0 },
            subrs_block: Record { start: 0, end: 0 },
            glyphs: Vec::new(),
            charstrings_header: Record { start: 0, end: 0 },
            charstrings_block: Record { start: 0, end: 0 },
            reader: Vec::new(),
            subr_terminator: Vec::new(),
            glyph_terminator: Vec::new(),
            font_name: None,
            font_bbox: None,
            italic_angle: None,
            std_vw: None,
        };
        font.parse_clear()?;
        font.parse_private()?;
        Ok(font)
    }

    fn parse_clear(&mut self) -> R<()> {
        let c = &self.clear;
        if let Some(at) = find(c, b"/FontName", 0)
            && let Some((s, e)) = token(c, at + 9)
            && c[s] == b'/'
        {
            self.font_name = Some(String::from_utf8_lossy(&c[s + 1..e]).into_owned());
        }
        if let Some(at) = find(c, b"/FontBBox", 0) {
            let mut i = at + 9;
            while i < c.len() && (is_ws(c[i]) || c[i] == b'{' || c[i] == b'[') {
                i += 1;
            }
            // `{a b c d}` or `[a b c d]`, possibly followed by `readonly def`
            // without whitespace; cut at the closing bracket first.
            let close = c[i..]
                .iter()
                .position(|&b| b == b'}' || b == b']')
                .map(|p| i + p)
                .ok_or(Type1Error::Truncated("FontBBox"))?;
            let inner = String::from_utf8_lossy(&c[i..close]).into_owned();
            let mut vals = Vec::new();
            for t in inner.split_ascii_whitespace() {
                vals.push(
                    t.parse::<f64>()
                        .map_err(|_| Type1Error::Malformed(format!("FontBBox value {t:?}")))?
                        .round() as i32,
                );
            }
            if vals.len() != 4 {
                return Err(Type1Error::Malformed(format!(
                    "FontBBox has {} values",
                    vals.len()
                )));
            }
            self.font_bbox = Some([vals[0], vals[1], vals[2], vals[3]]);
        }
        if let Some(at) = find(c, b"/ItalicAngle", 0)
            && let Some((s, e)) = token(c, at + 12)
        {
            self.italic_angle = Some(String::from_utf8_lossy(&c[s..e]).into_owned());
        }
        if let Some(at) = find(c, b"/FontMatrix", 0) {
            let line_end = find(c, b"\n", at).unwrap_or(c.len());
            let m: String = String::from_utf8_lossy(&c[at..line_end])
                .chars()
                .filter(|ch| !ch.is_whitespace())
                .collect();
            if !m.starts_with("/FontMatrix[0.001000.00100]") {
                return Err(Type1Error::Unsupported(format!(
                    "FontMatrix other than [0.001 0 0 0.001 0 0]: {}",
                    String::from_utf8_lossy(&c[at..line_end]).trim()
                )));
            }
        }
        Ok(())
    }

    fn parse_private(&mut self) -> R<()> {
        let p = &self.plain;
        if let Some(at) = find(p, b"/lenIV", 0) {
            let (v, _) = int_token(p, at + 6, "lenIV")?;
            if !(0..=32).contains(&v) {
                return Err(Type1Error::Unsupported(format!("lenIV {v}")));
            }
            self.len_iv = v as usize;
        }
        if let Some(at) = find(p, b"/StdVW", 0)
            && let Some(open) = find(p, b"[", at)
            && open < at + 12
            && let Some(close) = find(p, b"]", open)
        {
            let inner = String::from_utf8_lossy(&p[open + 1..close]).into_owned();
            self.std_vw = inner
                .split_ascii_whitespace()
                .next()
                .and_then(|t| t.parse::<f64>().ok())
                .map(|v| v.round() as i32);
        }
        // /Subrs n array
        let subrs_at =
            find(p, b"/Subrs", 0).ok_or_else(|| Type1Error::Unsupported("no /Subrs".into()))?;
        let (count, mut pos) = int_token(p, subrs_at + 6, "Subrs count")?;
        if !(0..=65536).contains(&count) {
            return Err(Type1Error::Limit("subroutine count"));
        }
        let (as_, ae) = token(p, pos).ok_or(Type1Error::Truncated("Subrs array"))?;
        if &p[as_..ae] != b"array" {
            return Err(Type1Error::Malformed(
                "/Subrs not followed by n array".into(),
            ));
        }
        self.subrs_header = Record {
            start: subrs_at,
            end: ae,
        };
        pos = ae;
        let mut subrs: Vec<Option<(Record, Record)>> = vec![None; count as usize];
        let mut block_start = None;
        let mut block_end = ae;
        loop {
            let Some((ts, te)) = token(p, pos) else {
                return Err(Type1Error::Truncated("Subrs entries"));
            };
            if &p[ts..te] != b"dup" {
                break;
            }
            let (index, after_index) = int_token(p, te, "Subrs index")?;
            let (len, after_len) = int_token(p, after_index, "Subrs length")?;
            let (rs, re) = token(p, after_len).ok_or(Type1Error::Truncated("RD"))?;
            if self.reader.is_empty() {
                self.reader = p[rs..re].to_vec();
            } else if self.reader != p[rs..re] {
                return Err(Type1Error::Malformed(
                    "reader procedure name changes".into(),
                ));
            }
            if len < 0 || re + 1 + len as usize > p.len() {
                return Err(Type1Error::Truncated("subroutine bytes"));
            }
            let bin = Record {
                start: re + 1,
                end: re + 1 + len as usize,
            };
            let (ns, ne) = token(p, bin.end).ok_or(Type1Error::Truncated("NP"))?;
            if self.subr_terminator.is_empty() {
                self.subr_terminator = p[ns..ne].to_vec();
            } else if self.subr_terminator != p[ns..ne] {
                return Err(Type1Error::Malformed(
                    "subroutine terminator changes".into(),
                ));
            }
            if index < 0 || index as usize >= subrs.len() {
                return Err(Type1Error::Malformed(format!(
                    "Subrs index {index} out of range"
                )));
            }
            if subrs[index as usize].is_some() {
                return Err(Type1Error::Malformed(format!(
                    "Subrs index {index} repeated"
                )));
            }
            subrs[index as usize] = Some((Record { start: ts, end: ne }, bin));
            block_start.get_or_insert(ts);
            block_end = ne;
            pos = ne;
        }
        self.subrs = subrs;
        self.subrs_block = Record {
            start: block_start.unwrap_or(ae),
            end: block_end,
        };
        // /CharStrings n dict dup begin
        let cs_at = find(p, b"/CharStrings", block_end)
            .ok_or_else(|| Type1Error::Unsupported("no /CharStrings after Subrs".into()))?;
        let (gcount, after_count) = int_token(p, cs_at + 12, "CharStrings count")?;
        if !(1..=65536).contains(&gcount) {
            return Err(Type1Error::Limit("glyph count"));
        }
        let mut pos = after_count;
        for expected in [&b"dict"[..], b"dup", b"begin"] {
            let (s, e) = token(p, pos).ok_or(Type1Error::Truncated("CharStrings header"))?;
            if &p[s..e] != expected {
                return Err(Type1Error::Malformed(format!(
                    "CharStrings header: expected {:?}, found {:?}",
                    String::from_utf8_lossy(expected),
                    String::from_utf8_lossy(&p[s..e])
                )));
            }
            pos = e;
        }
        self.charstrings_header = Record {
            start: cs_at,
            end: pos,
        };
        let entries_start = pos;
        let mut entries_end = pos;
        loop {
            let Some((ts, te)) = token(p, pos) else {
                return Err(Type1Error::Truncated("CharStrings entries"));
            };
            if p[ts] != b'/' {
                if &p[ts..te] == b"end" {
                    break;
                }
                return Err(Type1Error::Malformed(format!(
                    "CharStrings: unexpected token {:?}",
                    String::from_utf8_lossy(&p[ts..te])
                )));
            }
            let name = String::from_utf8_lossy(&p[ts + 1..te]).into_owned();
            let (len, after_len) = int_token(p, te, "charstring length")?;
            let (rs, re) = token(p, after_len).ok_or(Type1Error::Truncated("RD"))?;
            if self.reader != p[rs..re] {
                return Err(Type1Error::Malformed(
                    "charstring reader procedure differs from Subrs".into(),
                ));
            }
            if len < 0 || re + 1 + len as usize > p.len() {
                return Err(Type1Error::Truncated("charstring bytes"));
            }
            let bin = Record {
                start: re + 1,
                end: re + 1 + len as usize,
            };
            let (ns, ne) = token(p, bin.end).ok_or(Type1Error::Truncated("ND"))?;
            if self.glyph_terminator.is_empty() {
                self.glyph_terminator = p[ns..ne].to_vec();
            } else if self.glyph_terminator != p[ns..ne] {
                return Err(Type1Error::Malformed(
                    "charstring terminator changes".into(),
                ));
            }
            self.glyphs.push((name, Record { start: ts, end: ne }, bin));
            entries_end = ne;
            pos = ne;
        }
        if self.glyphs.len() > 65536 {
            return Err(Type1Error::Limit("glyph count"));
        }
        self.charstrings_block = Record {
            start: entries_start,
            end: entries_end,
        };
        Ok(())
    }

    pub fn font_name(&self) -> Option<&str> {
        self.font_name.as_deref()
    }

    pub fn font_bbox(&self) -> Option<[i32; 4]> {
        self.font_bbox
    }

    pub fn italic_angle(&self) -> Option<&str> {
        self.italic_angle.as_deref()
    }

    pub fn std_vw(&self) -> Option<i32> {
        self.std_vw
    }

    pub fn len_iv(&self) -> usize {
        self.len_iv
    }

    pub fn glyph_names(&self) -> impl Iterator<Item = &str> {
        self.glyphs.iter().map(|(n, _, _)| n.as_str())
    }

    pub fn subr_count(&self) -> usize {
        self.subrs.len()
    }

    fn glyph_record(&self, name: &str) -> Option<Record> {
        self.glyphs.iter().find(|(n, _, _)| n == name).map(|g| g.2)
    }

    /// The encrypted charstring bytes of a glyph, as stored.
    pub fn encrypted_charstring(&self, name: &str) -> Option<&[u8]> {
        self.glyph_record(name).map(|r| &self.plain[r.start..r.end])
    }

    /// The decrypted charstring of a glyph (`lenIV` bytes removed).
    pub fn decrypted_charstring(&self, name: &str) -> Option<Vec<u8>> {
        self.encrypted_charstring(name)
            .map(|c| decrypt(c, CHARSTRING_R, self.len_iv))
    }

    /// The decrypted subroutine `i`, if defined.
    pub fn decrypted_subr(&self, i: usize) -> Option<Vec<u8>> {
        self.subrs
            .get(i)
            .copied()
            .flatten()
            .map(|(_, bin)| decrypt(&self.plain[bin.start..bin.end], CHARSTRING_R, self.len_iv))
    }

    /// The advance width from the charstring's `hsbw`/`sbw`, in character
    /// space units (1000/em for the supported FontMatrix), as an exact
    /// rational: Latin Modern computes widths with `div` (`4787 11 div`),
    /// which is why pdfTeX takes widths from the TFM instead; the caller
    /// chooses the width source and any rounding.
    pub fn advance_width(&self, name: &str) -> R<crate::exact::Ratio> {
        use crate::exact::Ratio;
        let cs = self
            .decrypted_charstring(name)
            .ok_or_else(|| Type1Error::MissingGlyph(name.into()))?;
        let mut stack: Vec<Ratio> = Vec::new();
        let mut i = 0;
        let bad =
            || Type1Error::Malformed(format!("/{name}: charstring does not start with hsbw/sbw"));
        while i < cs.len() {
            let b0 = cs[i];
            match b0 {
                32..=246 => {
                    stack.push(Ratio::int(b0 as i128 - 139));
                    i += 1;
                }
                247..=250 => {
                    let b1 = *cs.get(i + 1).ok_or(Type1Error::Truncated("charstring"))?;
                    stack.push(Ratio::int((b0 as i128 - 247) * 256 + b1 as i128 + 108));
                    i += 2;
                }
                251..=254 => {
                    let b1 = *cs.get(i + 1).ok_or(Type1Error::Truncated("charstring"))?;
                    stack.push(Ratio::int(-(b0 as i128 - 251) * 256 - b1 as i128 - 108));
                    i += 2;
                }
                255 => {
                    let w = cs
                        .get(i + 1..i + 5)
                        .ok_or(Type1Error::Truncated("charstring"))?;
                    stack.push(Ratio::int(
                        i32::from_be_bytes([w[0], w[1], w[2], w[3]]) as i128
                    ));
                    i += 5;
                }
                13 => {
                    // sbx wx hsbw
                    return stack.get(1).copied().ok_or_else(bad);
                }
                12 => {
                    let b1 = *cs.get(i + 1).ok_or(Type1Error::Truncated("charstring"))?;
                    match b1 {
                        7 => return stack.get(2).copied().ok_or_else(bad), // sbx sby wx wy sbw
                        12 => {
                            let b = stack.pop().ok_or_else(bad)?;
                            let a = stack.pop().ok_or_else(bad)?;
                            if b.num == 0 {
                                return Err(Type1Error::Malformed(format!("/{name}: div by zero")));
                            }
                            stack.push(a / b);
                            i += 2;
                        }
                        _ => return Err(bad()),
                    }
                }
                _ => return Err(bad()),
            }
        }
        Err(Type1Error::Truncated("charstring"))
    }

    /// Walks a decrypted charstring, collecting subroutine indices and
    /// `seac` component names it reaches.
    fn scan(
        &self,
        cs: &[u8],
        subrs: &mut BTreeSet<usize>,
        seac: &mut Vec<String>,
        depth: usize,
    ) -> R<()> {
        if depth > 16 {
            return Err(Type1Error::Malformed(
                "subroutine nesting deeper than 16".into(),
            ));
        }
        let mut stack: Vec<i32> = Vec::new();
        let mut pending_pop: Vec<i32> = Vec::new();
        let mut i = 0;
        while i < cs.len() {
            let b0 = cs[i];
            match b0 {
                32..=246 => {
                    stack.push(b0 as i32 - 139);
                    i += 1;
                }
                247..=250 => {
                    let b1 = *cs.get(i + 1).ok_or(Type1Error::Truncated("charstring"))?;
                    stack.push((b0 as i32 - 247) * 256 + b1 as i32 + 108);
                    i += 2;
                }
                251..=254 => {
                    let b1 = *cs.get(i + 1).ok_or(Type1Error::Truncated("charstring"))?;
                    stack.push(-(b0 as i32 - 251) * 256 - b1 as i32 - 108);
                    i += 2;
                }
                255 => {
                    let w = cs
                        .get(i + 1..i + 5)
                        .ok_or(Type1Error::Truncated("charstring"))?;
                    stack.push(i32::from_be_bytes([w[0], w[1], w[2], w[3]]));
                    i += 5;
                }
                10 => {
                    // callsubr
                    let n = stack
                        .pop()
                        .ok_or_else(|| Type1Error::Malformed("callsubr without index".into()))?;
                    let n = usize::try_from(n)
                        .map_err(|_| Type1Error::Malformed("negative subr index".into()))?;
                    if subrs.insert(n) {
                        let body = self.decrypted_subr(n).ok_or_else(|| {
                            Type1Error::Malformed(format!("Subrs[{n}] is undefined"))
                        })?;
                        self.scan(&body, subrs, seac, depth + 1)?;
                    }
                    i += 1;
                }
                11 => return Ok(()), // return
                14 => return Ok(()), // endchar
                12 => {
                    let b1 = *cs.get(i + 1).ok_or(Type1Error::Truncated("charstring"))?;
                    match b1 {
                        16 => {
                            // arg1 … argn n othersubr# callothersubr
                            let other = stack.pop().unwrap_or(-1);
                            let n = stack.pop().unwrap_or(0).max(0) as usize;
                            let args: Vec<i32> =
                                stack.drain(stack.len().saturating_sub(n)..).collect();
                            if other == 3 {
                                // Hint replacement: the subroutine number comes back with `pop`.
                                if let Some(&subr) = args.first() {
                                    pending_pop.push(subr);
                                    if let Ok(s) = usize::try_from(subr)
                                        && subrs.insert(s)
                                    {
                                        let body = self.decrypted_subr(s).ok_or_else(|| {
                                            Type1Error::Malformed(format!(
                                                "Subrs[{s}] is undefined"
                                            ))
                                        })?;
                                        self.scan(&body, subrs, seac, depth + 1)?;
                                    }
                                }
                            } else {
                                // Flex and others hand their arguments back through `pop`.
                                pending_pop = args.into_iter().rev().collect();
                            }
                        }
                        17 => {
                            // pop
                            stack.push(pending_pop.pop().unwrap_or(0));
                        }
                        6 => {
                            // asb adx ady bchar achar seac
                            let achar = stack.pop().unwrap_or(-1);
                            let bchar = stack.pop().unwrap_or(-1);
                            for code in [bchar, achar] {
                                let name = standard_encoding_name(code).ok_or_else(|| {
                                    Type1Error::Malformed(format!(
                                        "seac code {code} is not in StandardEncoding"
                                    ))
                                })?;
                                seac.push(name.to_string());
                            }
                            return Ok(());
                        }
                        _ => stack.clear(),
                    }
                    i += 2;
                }
                _ => {
                    stack.clear();
                    i += 1;
                }
            }
        }
        Ok(())
    }

    /// Glyph names and subroutine indices a set of glyphs needs, including
    /// `seac` components (transitively) and Subrs 0–3 (Flex/hint
    /// replacement conventions) when the font defines them.
    pub fn closure(&self, names: &BTreeSet<String>) -> R<(BTreeSet<String>, BTreeSet<usize>)> {
        let mut glyphs: BTreeSet<String> = BTreeSet::new();
        let mut subrs: BTreeSet<usize> = BTreeSet::new();
        let mut queue: Vec<String> = names.iter().cloned().collect();
        queue.push(".notdef".into());
        while let Some(name) = queue.pop() {
            if glyphs.contains(&name) {
                continue;
            }
            let cs = self
                .decrypted_charstring(&name)
                .ok_or_else(|| Type1Error::MissingGlyph(name.clone()))?;
            glyphs.insert(name);
            let mut seac = Vec::new();
            self.scan(&cs, &mut subrs, &mut seac, 0)?;
            queue.extend(seac);
        }
        // Frozen old engine (D13): the index is the Subrs number kept.
        #[allow(clippy::needless_range_loop)]
        for i in 0..4.min(self.subrs.len()) {
            if self.subrs[i].is_some() {
                subrs.insert(i);
            }
        }
        Ok((glyphs, subrs))
    }

    /// Builds the subset program described in the module docs.
    pub fn subset(&self, names: &BTreeSet<String>) -> R<Type1Subset> {
        let (glyphs, subrs) = self.closure(names)?;
        let p = &self.plain;
        let mut out = Vec::with_capacity(p.len());
        // Everything up to the first Subrs entry, verbatim (includes the
        // `/Subrs n array` header: the count is unchanged since indices are).
        out.extend_from_slice(&p[..self.subrs_block.start]);
        // A blanked subroutine: lenIV bytes + `return`, charstring-encrypted.
        let blank = encrypt(&[11], CHARSTRING_R, &vec![0u8; self.len_iv]);
        let mut first = true;
        let mut retained_subrs = 0usize;
        for (i, entry) in self.subrs.iter().enumerate() {
            let Some((_, bin)) = entry else {
                continue;
            };
            if !first {
                out.push(b'\n');
            }
            first = false;
            let bytes: &[u8] = if subrs.contains(&i) {
                retained_subrs += 1;
                &p[bin.start..bin.end]
            } else {
                &blank
            };
            out.extend_from_slice(b"dup ");
            out.extend_from_slice(i.to_string().as_bytes());
            out.push(b' ');
            out.extend_from_slice(bytes.len().to_string().as_bytes());
            out.push(b' ');
            out.extend_from_slice(&self.reader);
            out.push(b' ');
            out.extend_from_slice(bytes);
            out.push(b' ');
            out.extend_from_slice(&self.subr_terminator);
        }
        // Between the Subrs block and the CharStrings header, verbatim.
        out.extend_from_slice(&p[self.subrs_block.end..self.charstrings_header.start]);
        // `/CharStrings n dict dup begin` with the retained count.
        out.extend_from_slice(b"/CharStrings ");
        out.extend_from_slice(glyphs.len().to_string().as_bytes());
        out.extend_from_slice(b" dict dup begin");
        let mut kept: Vec<(String, usize)> = Vec::new();
        for (name, _, bin) in &self.glyphs {
            if !glyphs.contains(name) {
                continue;
            }
            out.push(b'\n');
            out.push(b'/');
            out.extend_from_slice(name.as_bytes());
            out.push(b' ');
            out.extend_from_slice((bin.end - bin.start).to_string().as_bytes());
            out.push(b' ');
            out.extend_from_slice(&self.reader);
            out.push(b' ');
            out.extend_from_slice(&p[bin.start..bin.end]);
            out.push(b' ');
            out.extend_from_slice(&self.glyph_terminator);
            kept.push((name.clone(), bin.end - bin.start));
        }
        // From the end of the last original entry (the `end …` that follows), verbatim.
        out.extend_from_slice(&p[self.charstrings_block.end..]);
        let binary = encrypt(&out, EEXEC_R, &[0, 0, 0, 0]);
        let mut bytes = self.clear.clone();
        let length1 = bytes.len();
        bytes.extend_from_slice(&binary);
        Ok(Type1Subset {
            program: FontProgram::Type1 {
                bytes,
                length1,
                length2: binary.len(),
                length3: 0,
            },
            glyphs: kept.into_iter().map(|(n, _)| n).collect(),
            subrs_retained: retained_subrs,
            subrs_total: self.subrs.iter().filter(|s| s.is_some()).count(),
        })
    }

    /// The trailer bytes (`Length3` portion) of the source, if any.
    pub fn trailer(&self) -> &[u8] {
        &self.trailer
    }
}

/// The result of [`Type1Font::subset`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type1Subset {
    pub program: FontProgram,
    /// Glyph names retained, in the font's CharStrings order (`.notdef` and
    /// `seac` components included).
    pub glyphs: Vec<String>,
    pub subrs_retained: usize,
    pub subrs_total: usize,
}

/// Glyph name for a StandardEncoding code (Adobe Type 1 Font Format,
/// Appendix E), used by `seac`.
pub fn standard_encoding_name(code: i32) -> Option<&'static str> {
    use crate::cff::STANDARD_STRING_NAMES as N;
    let sid: usize = match code {
        32..=126 => (code - 31) as usize,
        161..=175 => (code - 161 + 96) as usize,
        177..=180 => (code - 177 + 111) as usize,
        182..=189 => (code - 182 + 115) as usize,
        191 => 123,
        193..=200 => (code - 193 + 124) as usize,
        202 | 203 => (code - 202 + 132) as usize,
        205..=208 => (code - 205 + 134) as usize,
        225 => 138,
        227 => 139,
        232..=235 => (code - 232 + 140) as usize,
        241 => 144,
        245 => 145,
        248..=251 => (code - 248 + 146) as usize,
        _ => return None,
    };
    N.get(sid).copied()
}

/// Glyph name to StandardEncoding code, the inverse of [`standard_encoding_name`].
pub fn standard_encoding_code(name: &str) -> Option<u8> {
    (0..=255).find(|&c| standard_encoding_name(c as i32) == Some(name))
}

/// Which glyph names a font's clear-text `/Encoding` assigns to codes
/// (`dup code /name put` entries); `StandardEncoding` expands to the table.
pub fn builtin_encoding(clear: &[u8]) -> BTreeMap<u8, String> {
    let mut map = BTreeMap::new();
    let Some(at) = find(clear, b"/Encoding", 0) else {
        return map;
    };
    if find(clear, b"StandardEncoding", at).is_some_and(|p| p < at + 30) {
        for c in 0..=255u8 {
            if let Some(n) = standard_encoding_name(c as i32) {
                map.insert(c, n.to_string());
            }
        }
        return map;
    }
    let end = find(clear, b"readonly def", at).unwrap_or(clear.len());
    let mut pos = at;
    while let Some(d) = find(clear, b"dup ", pos) {
        if d >= end {
            break;
        }
        pos = d + 4;
        // `dup 32/space put` or `dup 32 /space put`.
        let Some((ts, te)) = token(clear, pos) else {
            break;
        };
        let t = String::from_utf8_lossy(&clear[ts..te]).into_owned();
        let (code, name) = match t.split_once('/') {
            Some((c, n)) if !n.is_empty() => (c.to_string(), n.to_string()),
            _ => {
                let Some((ns, ne)) = token(clear, te) else {
                    break;
                };
                if clear[ns] != b'/' {
                    continue;
                }
                (
                    t.clone(),
                    String::from_utf8_lossy(&clear[ns + 1..ne]).into_owned(),
                )
            }
        };
        if let Ok(code) = code.parse::<u16>()
            && code <= 255
        {
            map.insert(code as u8, name);
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eexec_round_trip_and_spec_vector() {
        let plain = b"/Private 1 dict def\n";
        let enc = encrypt(plain, EEXEC_R, &[0, 1, 2, 3]);
        assert_eq!(decrypt(&enc, EEXEC_R, 4), plain);
        // font-resources' synthetic vector: d9d73f4a50b9fa428d59a36bd8f46f9cb9c47adfb0f3297e
        // decodes to prefix 00010203 then the line above.
        let vector: Vec<u8> = (0..24)
            .map(|i| {
                u8::from_str_radix(
                    &"d9d73f4a50b9fa428d59a36bd8f46f9cb9c47adfb0f3297e"[2 * i..2 * i + 2],
                    16,
                )
                .unwrap()
            })
            .collect();
        assert_eq!(enc, vector);
        let cs = encrypt(&[139, 13], CHARSTRING_R, &[0; 4]);
        assert_eq!(decrypt(&cs, CHARSTRING_R, 4), [139, 13]);
    }

    #[test]
    fn standard_encoding_table() {
        assert_eq!(standard_encoding_name(65), Some("A"));
        assert_eq!(standard_encoding_name(32), Some("space"));
        assert_eq!(standard_encoding_name(126), Some("asciitilde"));
        assert_eq!(standard_encoding_name(161), Some("exclamdown"));
        assert_eq!(standard_encoding_name(174), Some("fi"));
        assert_eq!(standard_encoding_name(177), Some("endash"));
        assert_eq!(standard_encoding_name(193), Some("grave"));
        assert_eq!(standard_encoding_name(200), Some("dieresis"));
        assert_eq!(standard_encoding_name(208), Some("emdash"));
        assert_eq!(standard_encoding_name(225), Some("AE"));
        assert_eq!(standard_encoding_name(251), Some("germandbls"));
        assert_eq!(standard_encoding_name(127), None);
        assert_eq!(standard_encoding_code("acute"), Some(194));
    }
}
