//! TFM fonts in Unicode mode's output: which font program draws a TFM
//! font's characters, from the font map (`pdftex.map`, which TeX Live's
//! `dvipdfmx.cfg` names), its encoding (`.enc`), slant and extension.
//!
//! The map's syntax is pdfTeX's (the pdfTeX manual, "Map files"): a line
//! is `tfmname [psname] ["ps code"] [<file.enc] [<file.pfb]`, with `<<`
//! (include whole) and `<[` (encoding) prefixes; `SlantFont` and
//! `ExtendFont` in the quoted code. Lines starting with `%`, `#`, `*`, `;`
//! or blank are comments.

use std::collections::HashMap;

/// One map entry.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MapEntry {
    pub tfm: Vec<u8>,
    pub ps_name: Option<Vec<u8>>,
    pub enc_file: Option<Vec<u8>>,
    pub font_file: Option<Vec<u8>>,
    /// SlantFont and ExtendFont, ×1000 (0: none).
    pub slant: i32,
    pub extend: i32,
}

/// A font map: the first entry for each TFM name wins (a later line for a
/// name already mapped is ignored).
#[derive(Clone, Debug, Default)]
pub struct FontMap {
    pub entries: HashMap<Vec<u8>, MapEntry>,
}

impl FontMap {
    pub fn parse(data: &[u8]) -> FontMap {
        let mut m = FontMap::default();
        for line in data.split(|&c| c == b'\n' || c == b'\r') {
            if let Some(e) = parse_line(line) {
                m.entries.entry(e.tfm.clone()).or_insert(e);
            }
        }
        m
    }

    /// `x:fontmapline` / `pdf:mapline`: `+line` adds unless present,
    /// `=line` replaces, `-line` removes, a bare line adds.
    pub fn apply_line(&mut self, line: &[u8]) {
        let (mode, rest) = match line.first() {
            Some(b'+') => (b'+', &line[1..]),
            Some(b'=') => (b'=', &line[1..]),
            Some(b'-') => (b'-', &line[1..]),
            _ => (b'+', line),
        };
        let Some(e) = parse_line(rest) else {
            return;
        };
        match mode {
            b'-' => {
                self.entries.remove(&e.tfm);
            }
            b'=' => {
                self.entries.insert(e.tfm.clone(), e);
            }
            _ => {
                self.entries.entry(e.tfm.clone()).or_insert(e);
            }
        }
    }

    pub fn get(&self, tfm: &[u8]) -> Option<&MapEntry> {
        self.entries.get(tfm)
    }
}

fn parse_line(line: &[u8]) -> Option<MapEntry> {
    let first = *line.iter().find(|c| !c.is_ascii_whitespace())?;
    if matches!(first, b'%' | b'#' | b'*' | b';') {
        return None;
    }
    let mut e = MapEntry::default();
    let mut i = 0;
    let n = line.len();
    let mut words = 0;
    while i < n {
        while i < n && line[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= n {
            break;
        }
        match line[i] {
            b'"' => {
                let st = i + 1;
                let en = line[st..]
                    .iter()
                    .position(|&c| c == b'"')
                    .map_or(n, |p| st + p);
                parse_ps_code(&line[st..en], &mut e);
                i = (en + 1).min(n);
            }
            b'<' => {
                i += 1;
                while i < n && matches!(line[i], b'<' | b'[') {
                    i += 1;
                }
                while i < n && line[i].is_ascii_whitespace() {
                    i += 1;
                }
                let st = i;
                while i < n && !line[i].is_ascii_whitespace() {
                    i += 1;
                }
                let f = line[st..i].to_vec();
                if f.len() >= 4 && f[f.len() - 4..].eq_ignore_ascii_case(b".enc") {
                    e.enc_file = Some(f);
                } else if !f.is_empty() {
                    e.font_file = Some(f);
                }
            }
            _ => {
                let st = i;
                while i < n && !line[i].is_ascii_whitespace() {
                    i += 1;
                }
                let w = line[st..i].to_vec();
                match words {
                    0 => e.tfm = w,
                    1 if w.iter().all(u8::is_ascii_digit) => {} // a dvips flag field
                    1 => e.ps_name = Some(w),
                    _ => {}
                }
                words += 1;
            }
        }
    }
    (!e.tfm.is_empty()).then_some(e)
}

fn parse_ps_code(code: &[u8], e: &mut MapEntry) {
    let toks: Vec<&[u8]> = code
        .split(|c| c.is_ascii_whitespace())
        .filter(|t| !t.is_empty())
        .collect();
    for (k, t) in toks.iter().enumerate() {
        let num = || {
            k.checked_sub(1)
                .and_then(|j| std::str::from_utf8(toks[j]).ok())
                .and_then(|s| s.parse::<f64>().ok())
        };
        match *t {
            b"SlantFont" => {
                if let Some(v) = num() {
                    e.slant = (v * 1000.0).round() as i32;
                }
            }
            b"ExtendFont" => {
                if let Some(v) = num() {
                    e.extend = (v * 1000.0).round() as i32;
                }
            }
            _ => {}
        }
    }
}

/// The 256 glyph names of an encoding file (`/Name [ /a /b ... ] def`).
pub fn parse_enc(data: &[u8]) -> Option<Vec<Vec<u8>>> {
    let mut out = vec![b".notdef".to_vec(); 256];
    let mut text = Vec::with_capacity(data.len());
    let mut in_comment = false;
    for &c in data {
        match c {
            b'%' => in_comment = true,
            b'\n' | b'\r' => {
                in_comment = false;
                text.push(b' ');
            }
            _ if !in_comment => text.push(c),
            _ => {}
        }
    }
    let open = text.iter().position(|&c| c == b'[')?;
    let close = open + text[open..].iter().position(|&c| c == b']')?;
    let mut i = 0;
    for tok in text[open + 1..close].split(|&c| c == b'/' || c.is_ascii_whitespace()) {
        if tok.is_empty() {
            continue;
        }
        if i < 256 {
            out[i] = tok.to_vec();
        }
        i += 1;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_lines() {
        let m = FontMap::parse(
            b"% comment\n\
              cmr10 CMR10 <cmr10.pfb\n\
              ec-lmr10 LMRoman10-Regular \"enclmec ReEncodeFont\" <lm-ec.enc <lmr10.pfb\n\
              rptmo Times-Roman \".167 SlantFont\" <8r.enc <utmr8a.pfb\n\
              pcrr8rn Courier \" .82 ExtendFont TeXBase1Encoding ReEncodeFont \" <8r.enc <<ucrr8a.pfb\n\
              cmr10 OTHER <other.pfb\n",
        );
        let e = m.get(b"cmr10").unwrap();
        assert_eq!(e.ps_name.as_deref(), Some(&b"CMR10"[..]));
        assert_eq!(e.font_file.as_deref(), Some(&b"cmr10.pfb"[..]));
        let e = m.get(b"ec-lmr10").unwrap();
        assert_eq!(e.enc_file.as_deref(), Some(&b"lm-ec.enc"[..]));
        assert_eq!(m.get(b"rptmo").unwrap().slant, 167);
        let e = m.get(b"pcrr8rn").unwrap();
        assert_eq!(e.extend, 820);
        assert_eq!(e.font_file.as_deref(), Some(&b"ucrr8a.pfb"[..]));
        let mut m = m;
        m.apply_line(b"=cmr10 CMR10X <x.pfb");
        assert_eq!(
            m.get(b"cmr10").unwrap().font_file.as_deref(),
            Some(&b"x.pfb"[..])
        );
        m.apply_line(b"-cmr10");
        assert!(m.get(b"cmr10").is_none());
    }

    #[test]
    fn enc_files() {
        let e = parse_enc(b"% x\n/Enc [ /a/b %c\n /c ] def").unwrap();
        assert_eq!(e[0], b"a");
        assert_eq!(e[2], b"c");
        assert_eq!(e[3], b".notdef");
    }
}
