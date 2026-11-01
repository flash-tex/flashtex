//! Strict passive private-dictionary record subset; no PostScript execution.
use crate::{eexec::Inspection, pfb::Identity, sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};
const MAX_BYTES: usize = 8 * 1024 * 1024;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Truncated,
    Unsupported(&'static str),
    Budget,
    Duplicate,
    Length,
    Missing,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub encrypted_range: Range<usize>,
}
struct Parsed {
    len_iv: u8,
    glyphs: BTreeMap<String, Record>,
    subrs: Vec<Option<Record>>,
    opaque_other_subrs: bool,
}
pub struct Records<'a> {
    inspection: &'a Inspection<'a>,
    parsed: Parsed,
}
impl Records<'_> {
    pub fn identity(&self) -> &Identity {
        self.inspection.resource_identity()
    }
    pub fn len_iv(&self) -> u8 {
        self.parsed.len_iv
    }
    pub fn glyphs(&self) -> &BTreeMap<String, Record> {
        &self.parsed.glyphs
    }
    pub fn subrs(&self) -> &[Option<Record>] {
        &self.parsed.subrs
    }
    pub fn opaque_other_subrs_present(&self) -> bool {
        self.parsed.opaque_other_subrs
    }
    pub fn decrypted_glyph(&self, name: &str) -> Result<Vec<u8>, Error> {
        let r = self.parsed.glyphs.get(name).ok_or(Error::Missing)?;
        decode(self.inspection.plaintext(), r, self.parsed.len_iv)
    }
    pub fn decrypted_subr(&self, index: usize) -> Result<Vec<u8>, Error> {
        let r = self
            .parsed
            .subrs
            .get(index)
            .and_then(Option::as_ref)
            .ok_or(Error::Missing)?;
        decode(self.inspection.plaintext(), r, self.parsed.len_iv)
    }
}
pub fn extract<'a>(inspection: &'a Inspection<'a>) -> Result<Records<'a>, Error> {
    Ok(Records {
        inspection,
        parsed: parse(inspection.plaintext())?,
    })
}
fn decode(bytes: &[u8], record: &Record, len_iv: u8) -> Result<Vec<u8>, Error> {
    let bytes = bytes
        .get(record.encrypted_range.clone())
        .ok_or(Error::Length)?;
    if bytes.len() < usize::from(len_iv) {
        return Err(Error::Length);
    }
    let mut seed = 4330u16;
    let mut out = Vec::with_capacity(bytes.len() - usize::from(len_iv));
    for (i, c) in bytes.iter().copied().enumerate() {
        let p = crate::eexec::decrypt_byte(c, &mut seed);
        if i >= usize::from(len_iv) {
            out.push(p)
        }
    }
    Ok(out)
}
pub(crate) struct Parser<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) at: usize,
    pub(crate) start: usize,
    pub(crate) tokens: usize,
}
fn white(c: u8) -> bool {
    matches!(c, 0 | 9 | 10 | 12 | 13 | 32)
}
fn delimiter(c: u8) -> bool {
    white(c) || b"[]{}()<>/%".contains(&c)
}
impl<'a> Parser<'a> {
    pub(crate) fn token(&mut self) -> Result<&'a [u8], Error> {
        loop {
            while self.bytes.get(self.at).is_some_and(|c| white(*c)) {
                self.at += 1
            }
            if self.bytes.get(self.at) == Some(&b'%') {
                while self
                    .bytes
                    .get(self.at)
                    .is_some_and(|c| *c != b'\n' && *c != b'\r')
                {
                    self.at += 1
                }
            } else {
                break;
            }
        }
        self.start = self.at;
        self.tokens += 1;
        if self.tokens > 65536 {
            return Err(Error::Budget);
        }
        let first = *self.bytes.get(self.at).ok_or(Error::Truncated)?;
        self.at += 1;
        if !b"[]{}".contains(&first) {
            if b"()< >%".contains(&first) {
                return Err(Error::Unsupported("strings/hex/expressions"));
            }
            while self.bytes.get(self.at).is_some_and(|c| !delimiter(*c)) {
                self.at += 1
            }
        }
        if self.at - self.start > 256 {
            return Err(Error::Budget);
        }
        Ok(&self.bytes[self.start..self.at])
    }
    pub(crate) fn expect(&mut self, value: &[u8]) -> Result<(), Error> {
        if self.token()? != value {
            Err(Error::Unsupported("dictionary grammar"))
        } else {
            Ok(())
        }
    }
    pub(crate) fn words(&mut self, words: &[&[u8]]) -> Result<(), Error> {
        for w in words {
            self.expect(w)?
        }
        Ok(())
    }
    pub(crate) fn integer(&mut self, cap: usize) -> Result<usize, Error> {
        let t = self.token()?;
        if t.is_empty() || !t.iter().all(u8::is_ascii_digit) {
            return Err(Error::Unsupported("literal nonnegative integer required"));
        }
        let n = std::str::from_utf8(t)
            .map_err(|_| Error::Length)?
            .parse::<usize>()
            .map_err(|_| Error::Length)?;
        if n > cap {
            return Err(Error::Budget);
        }
        Ok(n)
    }
    fn binary(&mut self) -> Result<Record, Error> {
        let n = self.integer(1024 * 1024)?;
        self.expect(b"RD")?;
        if !self.bytes.get(self.at).is_some_and(|c| white(*c)) {
            return Err(Error::Length);
        }
        // RD consumes exactly one separating whitespace byte. Never skip payload bytes.
        self.at += 1;
        let start = self.at;
        self.at = self.at.checked_add(n).ok_or(Error::Length)?;
        if self.at > self.bytes.len() {
            return Err(Error::Truncated);
        }
        Ok(Record {
            encrypted_range: start..self.at,
        })
    }
}
pub(crate) fn number(t: &[u8]) -> bool {
    let t = if matches!(t.first(), Some(b'+' | b'-')) {
        &t[1..]
    } else {
        t
    };
    !t.is_empty()
        && t.iter().filter(|c| **c == b'.').count() <= 1
        && t.iter().any(u8::is_ascii_digit)
        && t.iter().all(|c| c.is_ascii_digit() || *c == b'.')
}
fn parse(bytes: &[u8]) -> Result<Parsed, Error> {
    if bytes.len() > MAX_BYTES {
        return Err(Error::Budget);
    }
    let mut p = Parser {
        bytes,
        at: 0,
        start: 0,
        tokens: 0,
    };
    p.words(&[b"dup", b"/Private"])?;
    let _capacity = p.integer(4096)?;
    p.words(&[b"dict", b"dup", b"begin"])?;
    p.words(&[
        b"/RD",
        b"{",
        b"string",
        b"currentfile",
        b"exch",
        b"readstring",
        b"pop",
        b"}",
        b"executeonly",
        b"def",
    ])?;
    p.words(&[
        b"/ND",
        b"{",
        b"noaccess",
        b"def",
        b"}",
        b"executeonly",
        b"def",
    ])?;
    p.words(&[
        b"/NP",
        b"{",
        b"noaccess",
        b"put",
        b"}",
        b"executeonly",
        b"def",
    ])?;
    let mut seen = BTreeSet::new();
    let mut len_iv = 4;
    let mut opaque_other_subrs = false;
    loop {
        let key = p.token()?;
        if key == b"/Subrs" {
            break;
        }
        if !seen.insert(key.to_vec()) {
            return Err(Error::Duplicate);
        }
        match key {
            b"/lenIV" => {
                len_iv = p.integer(32)? as u8;
                p.expect(b"def")?
            }
            b"/MinFeature" => {
                p.words(&[b"{", b"16", b"16", b"}"])?;
                if ![b"ND".as_slice(), b"def"].contains(&p.token()?) {
                    return Err(Error::Unsupported("MinFeature terminator"));
                }
            }
            b"/OtherSubrs" => {
                // Exact known LM declaration preserved as opaque data; never execute it.
                let end = p.start.checked_add(271).ok_or(Error::Length)?;
                let known = bytes.get(p.start..end).ok_or(Error::Truncated)?;
                if sha256(known)
                    != "2f4adc4e2d703495501ce0ee4cd3d955949139a5f8623779025d58a200e14c3a"
                {
                    return Err(Error::Unsupported("OtherSubrs executable form"));
                }
                p.at = end;
                opaque_other_subrs = true;
            }
            b"/BlueValues" | b"/OtherBlues" | b"/FamilyBlues" | b"/FamilyOtherBlues"
            | b"/StdHW" | b"/StdVW" | b"/StemSnapH" | b"/StemSnapV" => {
                p.expect(b"[")?;
                let mut count = 0;
                loop {
                    let t = p.token()?;
                    if t == b"]" {
                        break;
                    }
                    count += 1;
                    if count > 128 || !number(t) {
                        return Err(Error::Unsupported("literal numeric array"));
                    }
                }
                p.expect(b"def")?
            }
            b"/BlueScale" | b"/BlueShift" | b"/BlueFuzz" | b"/password" | b"/UniqueID"
            | b"/LanguageGroup" | b"/ExpansionFactor" => {
                if !number(p.token()?) {
                    return Err(Error::Unsupported("literal numeric value"));
                }
                p.expect(b"def")?
            }
            b"/ForceBold" | b"/RndStemUp" => {
                if ![b"true".as_slice(), b"false"].contains(&p.token()?) {
                    return Err(Error::Unsupported("literal boolean"));
                }
                p.expect(b"def")?
            }
            _ => return Err(Error::Unsupported("private dictionary form")),
        }
    }
    let count = p.integer(4096)?;
    p.expect(b"array")?;
    let mut subrs = vec![None; count];
    loop {
        let t = p.token()?;
        if t == b"ND" {
            break;
        }
        if t != b"dup" {
            return Err(Error::Unsupported("Subrs assignment"));
        }
        let index = p.integer(4095)?;
        if index >= count {
            return Err(Error::Length);
        }
        if subrs[index].is_some() {
            return Err(Error::Duplicate);
        }
        subrs[index] = Some(p.binary()?);
        p.expect(b"NP")?;
    }
    p.words(&[b"2", b"index", b"/CharStrings"])?;
    let count = p.integer(4096)?;
    p.words(&[b"dict", b"dup", b"begin"])?;
    let mut glyphs = BTreeMap::new();
    loop {
        let name = p.token()?;
        if name == b"end" {
            break;
        }
        if name.first() != Some(&b'/')
            || name.len() < 2
            || !name[1..].iter().all(|c| (33..=126).contains(c))
        {
            return Err(Error::Unsupported("literal glyph name"));
        }
        if glyphs.len() == count {
            return Err(Error::Budget);
        }
        let name = std::str::from_utf8(&name[1..])
            .map_err(|_| Error::Length)?
            .to_owned();
        let record = p.binary()?;
        if glyphs.insert(name, record).is_some() {
            return Err(Error::Duplicate);
        }
        p.expect(b"ND")?;
    }
    p.words(&[b"end", b"readonly", b"put"])?;
    let t = p.token()?;
    if t == b"noaccess" {
        p.expect(b"put")?
    } else if t != b"put" {
        return Err(Error::Unsupported("dictionary closure"));
    }
    p.words(&[
        b"dup",
        b"/FontName",
        b"get",
        b"exch",
        b"definefont",
        b"pop",
        b"mark",
        b"currentfile",
        b"closefile",
    ])?;
    if bytes[p.at..].iter().any(|c| !white(*c)) {
        return Err(Error::Unsupported("trailing executable code"));
    }
    for r in glyphs.values().chain(subrs.iter().flatten()) {
        if r.encrypted_range.len() < usize::from(len_iv) {
            return Err(Error::Length);
        }
    }
    Ok(Parsed {
        len_iv,
        glyphs,
        subrs,
        opaque_other_subrs,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(data: &[u8], len_iv: i32) -> Vec<u8> {
        let mut b=format!("dup /Private 4 dict dup begin /RD {{string currentfile exch readstring pop}} executeonly def /ND {{noaccess def}} executeonly def /NP {{noaccess put}} executeonly def /lenIV {len_iv} def /Subrs 1 array dup 0 {} RD ",data.len()).into_bytes();
        b.extend(data);
        b.extend(
            format!(
                " NP ND 2 index /CharStrings 1 dict dup begin /A {} RD ",
                data.len()
            )
            .as_bytes(),
        );
        b.extend(data);
        b.extend(b" ND end end readonly put put dup /FontName get exch definefont pop mark currentfile closefile\n");
        b
    }
    #[test]
    fn binary_delimiters_are_not_tokens_and_records_preserve_ranges() {
        let payload = b" /CharStrings { } %\nND RD \x00\xff";
        let bytes = fixture(payload, 0);
        let p = parse(&bytes).unwrap();
        assert_eq!(p.len_iv, 0);
        let g = p.glyphs.get("A").unwrap();
        assert_eq!(&bytes[g.encrypted_range.clone()], payload);
        assert_eq!(
            &bytes[p.subrs[0].as_ref().unwrap().encrypted_range.clone()],
            payload
        );
        for at in 0..bytes.len() - 1 {
            assert!(parse(&bytes[..at]).is_err(), "{at}")
        }
    }
    #[test]
    fn unknown_readers_executable_forms_and_lengths_refuse() {
        let base = fixture(b"1234", 4);
        let text = String::from_utf8(base).unwrap();
        for changed in [
            text.replace("readstring pop", "readstring exec"),
            text.replace("/lenIV 4", "/lenIV -1"),
            text.replace("/lenIV 4", "/lenIV 33"),
            text.replace("/Subrs 1", "/Subrs 4097"),
            text.replace("dup 0 4 RD", "dup 1 4 RD"),
            text.replace("/A 4 RD", "/A 999 RD"),
            text.replace("/lenIV 4 def", "/lenIV 4 def /lenIV 4 def"),
            text.replace("/lenIV 4 def", "/foo {exec} def"),
            text.clone() + " exec",
        ] {
            assert!(parse(changed.as_bytes()).is_err())
        }
    }
    #[test]
    fn adobe_published_charstring_vector() {
        // Adobe Type1 section7.3 example ciphertext; public format vector, seed4330/prefix4.
        let cipher =
            "10BF31704FAB5B1F03F9B68B1F39A66521B1841F1481697F8E12B7F7DDD6E3D7248D965B1CD45E2114";
        let plain = "BDF9B40D8BEF038BEF01F8ECEF018B16F95006EF07FCEC06F88807F8EC06EF07FD5006090E";
        let hex = |s: &str| {
            (0..s.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
                .collect::<Vec<_>>()
        };
        let c = hex(cipher);
        assert_eq!(
            decode(
                &c,
                &Record {
                    encrypted_range: 0..c.len()
                },
                4
            )
            .unwrap(),
            hex(plain)
        );
    }
}
