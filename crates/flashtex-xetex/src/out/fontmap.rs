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
use std::hash::BuildHasherDefault;

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
///
/// TeX Live's `pdftex.map` has 46,000 lines (5.5 MB) and a document uses
/// a few of them, so the map is indexed by TFM name when read and a line
/// is parsed only when its font is asked for ([`FontMap::get`]); what
/// `x:fontmapline` changes is kept beside the file's lines.
#[derive(Clone, Debug, Default)]
pub struct FontMap {
    data: Vec<u8>,
    /// The first line of `data` for each TFM name, as a range.
    index: HashMap<Box<[u8]>, (u32, u32), BuildHasherDefault<FxHasher>>,
    /// The entries `x:fontmapline` added or replaced, or removed (`None`).
    changed: HashMap<Vec<u8>, Option<MapEntry>>,
}

impl FontMap {
    pub fn parse(data: &[u8]) -> FontMap {
        FontMap::from_vec(data.to_vec())
    }

    /// [`FontMap::parse`] of a map file's bytes, kept.
    pub fn from_vec(data: Vec<u8>) -> FontMap {
        let mut index: HashMap<Box<[u8]>, (u32, u32), BuildHasherDefault<FxHasher>> =
            HashMap::with_capacity_and_hasher(data.len() / 100, Default::default());
        let mut start = 0;
        let n = data.len();
        while start <= n {
            let end = data[start..]
                .iter()
                .position(|&c| c == b'\n' || c == b'\r')
                .map_or(n, |p| start + p);
            if let Some(t) = tfm_of(&data[start..end]) {
                if !index.contains_key(t) {
                    index.insert(t.into(), (start as u32, end as u32));
                }
            }
            start = end + 1;
        }
        FontMap {
            data,
            index,
            changed: HashMap::new(),
        }
    }

    fn contains(&self, tfm: &[u8]) -> bool {
        match self.changed.get(tfm) {
            Some(e) => e.is_some(),
            None => self.index.contains_key(tfm),
        }
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
                self.changed.insert(e.tfm.clone(), None);
            }
            b'=' => {
                self.changed.insert(e.tfm.clone(), Some(e));
            }
            _ => {
                if !self.contains(&e.tfm) {
                    self.changed.insert(e.tfm.clone(), Some(e));
                }
            }
        }
    }

    pub fn get(&self, tfm: &[u8]) -> Option<MapEntry> {
        match self.changed.get(tfm) {
            Some(e) => e.clone(),
            None => {
                let &(a, b) = self.index.get(tfm)?;
                parse_line(&self.data[a as usize..b as usize])
            }
        }
    }
}

/// rustc's FxHasher: the index's keys are TFM names from a file TeX Live
/// writes, so a fast hash without DoS resistance is enough.
#[derive(Default)]
struct FxHasher(u64);

impl std::hash::Hasher for FxHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0.rotate_left(5) ^ b as u64).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
        }
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

/// The TFM name [`parse_line`] gives `line` (its first word that is not
/// quoted PostScript code or a `<` file), without parsing the rest; `None`
/// where it gives no entry.
fn tfm_of(line: &[u8]) -> Option<&[u8]> {
    let first = *line.iter().find(|c| !c.is_ascii_whitespace())?;
    if matches!(first, b'%' | b'#' | b'*' | b';') {
        return None;
    }
    let mut i = 0;
    let n = line.len();
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
                while i < n && !line[i].is_ascii_whitespace() {
                    i += 1;
                }
            }
            _ => {
                let st = i;
                while i < n && !line[i].is_ascii_whitespace() {
                    i += 1;
                }
                return Some(&line[st..i]);
            }
        }
    }
    None
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
        assert_eq!(e.tfm, b"cmr10");
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

    /// The map as it was read before the index: every line parsed, the
    /// first entry for each name kept, the lines applied in order.
    fn eager(data: &[u8], lines: &[&[u8]]) -> HashMap<Vec<u8>, MapEntry> {
        let mut m: HashMap<Vec<u8>, MapEntry> = HashMap::new();
        for line in data.split(|&c| c == b'\n' || c == b'\r') {
            if let Some(e) = parse_line(line) {
                m.entry(e.tfm.clone()).or_insert(e);
            }
        }
        for l in lines {
            let (mode, rest) = match l.first() {
                Some(b'+') => (b'+', &l[1..]),
                Some(b'=') => (b'=', &l[1..]),
                Some(b'-') => (b'-', &l[1..]),
                _ => (b'+', *l),
            };
            let Some(e) = parse_line(rest) else { continue };
            match mode {
                b'-' => {
                    m.remove(&e.tfm);
                }
                b'=' => {
                    m.insert(e.tfm.clone(), e);
                }
                _ => {
                    m.entry(e.tfm.clone()).or_insert(e);
                }
            }
        }
        m
    }

    #[test]
    fn the_index_answers_as_every_line_parsed() {
        let data: &[u8] = b"% c\n  \n\"x\" cmr5 CMR5 <cmr5.pfb\n<a.enc cmr6 CMR6\r\
            cmr7 CMR7 \"unterminated\n<< only.pfb\ncmr7 SECOND\n\tcmr8\t7 CMR8\n;x\n";
        let lines: [&[u8]; 6] = [
            b"+cmr5 NEW",
            b"=cmr6 REPL <r.pfb",
            b"-cmr7",
            b"+cmr7 BACK",
            b"cmr9 NINE",
            b"-nonesuch",
        ];
        let want = eager(data, &lines);
        let mut m = FontMap::parse(data);
        for l in lines {
            m.apply_line(l);
        }
        for name in [
            &b"cmr5"[..],
            b"cmr6",
            b"cmr7",
            b"cmr8",
            b"cmr9",
            b"nonesuch",
            b"only.pfb",
        ] {
            assert_eq!(
                m.get(name),
                want.get(name).cloned(),
                "{}",
                String::from_utf8_lossy(name)
            );
        }
        // TeX Live's own map, every name, where there is one
        let Some(p) = flashtex_engine::resolver::discover_texlive()
            .and_then(|t| {
                std::process::Command::new(t.bin.join("kpsewhich"))
                    .arg("pdftex.map")
                    .output()
                    .ok()
            })
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|p| !p.is_empty())
        else {
            return;
        };
        let data = std::fs::read(p).unwrap();
        let want = eager(&data, &[]);
        let m = FontMap::parse(&data);
        assert_eq!(m.index.len(), want.len());
        for (k, v) in &want {
            assert_eq!(m.get(k).as_ref(), Some(v));
        }
    }

    #[test]
    fn enc_files() {
        let e = parse_enc(b"% x\n/Enc [ /a/b %c\n /c ] def").unwrap();
        assert_eq!(e[0], b"a");
        assert_eq!(e[2], b"c");
        assert_eq!(e[3], b".notdef");
    }
}
