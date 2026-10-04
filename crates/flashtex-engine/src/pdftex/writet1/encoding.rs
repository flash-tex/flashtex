//! Encoding vectors: Adobe's StandardEncoding, the `[/name ...]` arrays
//! of `.enc` files and of a font's own `/Encoding`, and `.enc` files as
//! writet1.c's `load_enc_file` reads them.

use super::reader::{append_char, append_eol, without_eol};
use super::{Fail, Result};
use crate::pdftex::fonts::{notdef_names, GlyphNames, NOTDEF};

/// `standard_glyph_names` (writet1.c): Adobe StandardEncoding.
#[rustfmt::skip]
const STANDARD: [&[u8]; 256] = {
    const N: &[u8] = NOTDEF;
    [
        // 0x00
        N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N,
        // 0x10
        N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N,
        // 0x20
        b"space", b"exclam", b"quotedbl", b"numbersign", b"dollar", b"percent",
        b"ampersand", b"quoteright", b"parenleft", b"parenright", b"asterisk",
        b"plus", b"comma", b"hyphen", b"period", b"slash",
        // 0x30
        b"zero", b"one", b"two", b"three", b"four", b"five", b"six", b"seven",
        b"eight", b"nine", b"colon", b"semicolon", b"less", b"equal",
        b"greater", b"question",
        // 0x40
        b"at", b"A", b"B", b"C", b"D", b"E", b"F", b"G",
        b"H", b"I", b"J", b"K", b"L", b"M", b"N", b"O",
        // 0x50
        b"P", b"Q", b"R", b"S", b"T", b"U", b"V", b"W",
        b"X", b"Y", b"Z", b"bracketleft", b"backslash", b"bracketright",
        b"asciicircum", b"underscore",
        // 0x60
        b"quoteleft", b"a", b"b", b"c", b"d", b"e", b"f", b"g",
        b"h", b"i", b"j", b"k", b"l", b"m", b"n", b"o",
        // 0x70
        b"p", b"q", b"r", b"s", b"t", b"u", b"v", b"w",
        b"x", b"y", b"z", b"braceleft", b"bar", b"braceright", b"asciitilde", N,
        // 0x80
        N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N,
        // 0x90
        N, N, N, N, N, N, N, N, N, N, N, N, N, N, N, N,
        // 0xa0
        N, b"exclamdown", b"cent", b"sterling", b"fraction", b"yen", b"florin",
        b"section", b"currency", b"quotesingle", b"quotedblleft",
        b"guillemotleft", b"guilsinglleft", b"guilsinglright", b"fi", b"fl",
        // 0xb0
        N, b"endash", b"dagger", b"daggerdbl", b"periodcentered", N,
        b"paragraph", b"bullet", b"quotesinglbase", b"quotedblbase",
        b"quotedblright", b"guillemotright", b"ellipsis", b"perthousand", N,
        b"questiondown",
        // 0xc0
        N, b"grave", b"acute", b"circumflex", b"tilde", b"macron", b"breve",
        b"dotaccent", b"dieresis", N, b"ring", b"cedilla", N, b"hungarumlaut",
        b"ogonek", b"caron",
        // 0xd0
        b"emdash", N, N, N, N, N, N, N, N, N, N, N, N, N, N, N,
        // 0xe0
        N, b"AE", N, b"ordfeminine", N, N, N, N,
        b"Lslash", b"Oslash", b"OE", b"ordmasculine", N, N, N, N,
        // 0xf0
        N, b"ae", N, N, N, b"dotlessi", N, N,
        b"lslash", b"oslash", b"oe", b"germandbls", N, N, N, N,
    ]
};

/// The StandardEncoding name of code `code` (`.notdef` outside 0..=255).
pub(crate) fn standard_glyph_name(code: usize) -> &'static [u8] {
    STANDARD.get(code).copied().unwrap_or(NOTDEF)
}

/// StandardEncoding as a glyph-name vector.
pub(super) fn standard_names() -> GlyphNames {
    STANDARD.iter().map(|n| n.to_vec()).collect()
}

/// The names of an encoding array from `line[at..]`, `/a /b ...`, into
/// `names` from index `*count` on (`.notdef`s are left as they are), as far
/// as the first byte that does not start a name; returns that position.
/// More than 256 names is an error.
pub(super) fn read_names(
    line: &[u8],
    mut at: usize,
    names: &mut GlyphNames,
    count: &mut usize,
) -> Result<usize> {
    while line.get(at) == Some(&b'/') {
        at += 1;
        let len = line[at..]
            .iter()
            .take_while(|&&c| !matches!(c, b' ' | b'\n' | b']' | b'/'))
            .count();
        let name = &line[at..at + len];
        at += len;
        if line.get(at) == Some(&b' ') {
            at += 1;
        }
        if *count > 255 {
            return Err(Fail("encoding vector contains more than 256 names".into()));
        }
        if name != NOTDEF {
            names[*count] = name.to_vec();
        }
        *count += 1;
    }
    Ok(at)
}

/// The size of `enc_line_array` (writet1.c's `ENC_BUF_SIZE`).
const ENC_BUF_SIZE: usize = 0x1000;

/// The encoding vector of an `.enc` file (`load_enc_file` after the file
/// is open): `/Name [ /a /b ... ] def`, `%` comments, at most 256 names.
pub(super) fn parse_enc_file(data: &[u8]) -> Result<GlyphNames> {
    let mut lines = EncLines {
        data,
        pos: 0,
        eof: false,
    };
    let mut names = notdef_names();
    let mut line = lines.next()?;
    let bracket = line.iter().position(|&c| c == b'[');
    let (Some(bracket), Some(b'/')) = (bracket, line.first()) else {
        return Err(Fail(format!(
            "invalid encoding vector (a name or `[' missing): `{}'",
            without_eol(&line)
        )));
    };
    let mut count = 0;
    let mut at = bracket + 1;
    if line.get(at) == Some(&b' ') {
        at += 1;
    }
    loop {
        at = read_names(&line, at, &mut names, &mut count)?;
        let c = line.get(at).copied().unwrap_or(0);
        if c != b'\n' && c != b'%' {
            if line[at.min(line.len())..].starts_with(b"] def") {
                return Ok(names);
            }
            return Err(Fail(format!(
                "invalid encoding vector: a name or `] def' expected: `{}'",
                without_eol(&line)
            )));
        }
        line = lines.next()?;
        at = 0;
    }
}

/// `enc_getline`: the lines of an `.enc` file, normalised as a font's are,
/// without empty lines and `%` comment lines.
struct EncLines<'a> {
    data: &'a [u8],
    pos: usize,
    eof: bool,
}

impl EncLines<'_> {
    fn next(&mut self) -> Result<Vec<u8>> {
        loop {
            if self.eof {
                return Err(Fail("unexpected end of file".into()));
            }
            let mut line = Vec::new();
            loop {
                let c = self.data.get(self.pos).copied();
                match c {
                    Some(_) => self.pos += 1,
                    None => self.eof = true,
                }
                let c = append_char(c, &mut line);
                if line.len() + 1 > ENC_BUF_SIZE {
                    return Err(Fail("buffer overflow at file writet1.c, line 223".into()));
                }
                if c == b'\n' {
                    break;
                }
            }
            append_eol(&mut line);
            if line.len() >= 2 && line[0] != b'%' {
                return Ok(line);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_encoding() {
        assert_eq!(standard_glyph_name(0x41), b"A");
        assert_eq!(standard_glyph_name(0xfb), b"germandbls");
        assert_eq!(standard_glyph_name(0x7f), NOTDEF);
        assert_eq!(standard_glyph_name(256), NOTDEF);
        assert_eq!(STANDARD.iter().filter(|&&n| n != NOTDEF).count(), 149);
    }

    #[test]
    fn an_enc_file() {
        let enc = b"% comment\n/Test [ /A /.notdef\n/B % x\n  /C ] def\n";
        let names = parse_enc_file(enc).unwrap();
        assert_eq!(names[0], b"A");
        assert_eq!(names[1], NOTDEF);
        assert_eq!(names[2], b"B");
        assert_eq!(names[3], b"C");
        assert_eq!(names[4], NOTDEF);
    }

    #[test]
    fn enc_file_errors() {
        let e = parse_enc_file(b"Test [ /A ] def\n").unwrap_err();
        assert_eq!(
            e.0,
            "invalid encoding vector (a name or `[' missing): `Test [ /A ] def'"
        );
        let e = parse_enc_file(b"/Test [ /A ] readonly def\n").unwrap_err();
        assert!(e
            .0
            .starts_with("invalid encoding vector: a name or `] def' expected"));
        let e = parse_enc_file(b"/Test [ /A\n").unwrap_err();
        assert_eq!(e.0, "unexpected end of file");
        let many: String = (0..257).map(|i| format!("/g{i} ")).collect();
        let e = parse_enc_file(format!("/T [ {many}] def\n").as_bytes()).unwrap_err();
        assert_eq!(e.0, "encoding vector contains more than 256 names");
    }
}
