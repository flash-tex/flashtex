//! dvipdfmx's Adobe Glyph List handling (`agl.c` of TeX Live 2026's
//! `texk/dvipdfm-x`, GPL-2.0-or-later): the lists xdvipdfmx loads
//! (`agl_init_map`: `texglyphlist.txt`, then `pdfglyphlist.txt`, then
//! `glyphlist.txt`, found as kpathsea's `fontmap` files) and the two ways it
//! turns a glyph name into Unicode: `agl_get_unicodes` (the `ToUnicode` of an
//! OpenType or TrueType font's glyphs no `cmap` or `GSUB` reaches,
//! `tt_cmap.c`) and `agl_sput_UTF16BE` (a Type 1 font's encoding,
//! `pdfencoding.c`). Each routine names the C it ports.

use flashtex_engine::resolver::Format;
use std::collections::HashMap;

/// `AGL_EXTRA_LISTFILE`, `AGL_PREDEF_LISTFILE`, `AGL_DEFAULT_LISTFILE`, in
/// the order `agl_init_map` loads them.
pub const LIST_FILES: [&str; 3] = ["texglyphlist.txt", "pdfglyphlist.txt", "glyphlist.txt"];

/// `aglmap`: a glyph name's entry. A name several lines (or lists) give is
/// looked up as the first one loaded (`ht_lookup_table` finds the first;
/// later ones are only its `alternate`s).
#[derive(Default)]
pub struct Agl {
    map: HashMap<Vec<u8>, Vec<i32>>,
}

/// `modifiers[]`.
const MODIFIERS: [&[u8]; 20] = [
    b"acute",
    b"breve",
    b"caron",
    b"cedilla",
    b"circumflex",
    b"dieresis",
    b"dotaccent",
    b"grave",
    b"hungarumlaut",
    b"macron",
    b"ogonek",
    b"ring",
    b"tilde",
    b"commaaccent",
    b"slash",
    b"ampersand",
    b"exclam",
    b"exclamdown",
    b"question",
    b"questiondown",
];

/// The keys of `var_list[]` (their suffixes and OpenType tags are not read
/// on this path).
const VAR_KEYS: [&[u8]; 13] = [
    b"small",
    b"swash",
    b"superior",
    b"inferior",
    b"numerator",
    b"denominator",
    b"oldstyle",
    b"display",
    b"text",
    b"big",
    b"bigg",
    b"Big",
    b"Bigg",
];

impl Agl {
    /// `agl_init_map`: the lists the resolver finds (TeX Live's, or the
    /// bundle's); a list it does not find is left out, as xdvipdfmx warns
    /// and goes on.
    pub fn load() -> Agl {
        Agl::load_files(&Agl::find_files())
    }

    /// The paths of [`LIST_FILES`] as `load` finds them (`None`: not found).
    pub fn find_files() -> Vec<Option<String>> {
        LIST_FILES
            .iter()
            .map(|name| flashtex_engine::system::find_file(name, Format::Map))
            .collect()
    }

    /// The list of the files found (`find_files`), in order.
    pub fn load_files(paths: &[Option<String>]) -> Agl {
        let mut agl = Agl::default();
        for p in paths.iter().flatten() {
            if let Ok(data) = std::fs::read(p) {
                agl.load_list(&data);
            }
        }
        agl
    }

    /// `agl_load_listfile`: `name;XXXX XXXX ...` lines.
    pub fn load_list(&mut self, data: &[u8]) {
        for line in data.split(|&b| b == b'\n') {
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            let mut p = 0;
            let end = line.len();
            // skip_white (with `%` comments to the end of the line)
            while p < end && (is_space(line[p]) || line[p] == b'%') {
                if line[p] == b'%' {
                    p = end;
                } else {
                    p += 1;
                }
            }
            if p >= end || line[p] == b'#' {
                continue;
            }
            let Some(semi) = line[p..].iter().position(|&b| b == b';').map(|k| p + k) else {
                continue;
            };
            if semi == p {
                continue;
            }
            // parse_ident up to the `;`
            let start = p;
            while p < semi && is_ident_char(line[p]) {
                p += 1;
            }
            if p == start {
                continue;
            }
            let name = line[start..p].to_vec();
            while p < end && is_space(line[p]) {
                p += 1;
            }
            if p >= end || line[p] != b';' {
                continue; // "Invalid AGL entry"
            }
            p += 1;
            while p < end && is_space(line[p]) {
                p += 1;
            }
            let mut unicodes = Vec::new();
            while p < end && (line[p].is_ascii_digit() || (b'A'..=b'F').contains(&line[p])) {
                if unicodes.len() >= 64 {
                    break; // AGL_MAX_UNICODES: "Too many Unicode values"
                }
                // strtol(p, &nextptr, 16)
                let s = p;
                while p < end && line[p].is_ascii_hexdigit() {
                    p += 1;
                }
                let v = std::str::from_utf8(&line[s..p])
                    .ok()
                    .and_then(|h| i64::from_str_radix(h, 16).ok())
                    .unwrap_or(0);
                unicodes.push(v.clamp(i32::MIN as i64, i32::MAX as i64) as i32);
                while p < end && is_space(line[p]) {
                    p += 1;
                }
            }
            if unicodes.is_empty() {
                continue; // "AGL entry ignored (no mapping)"
            }
            self.map.entry(name).or_insert(unicodes);
        }
    }

    /// `agl_lookup_list`.
    fn lookup(&self, name: &[u8]) -> Option<&Vec<i32>> {
        self.map.get(name)
    }

    /// The entry of one name component: its own, unless it has none or a
    /// single private-use value, then its normalized name's.
    fn component(&self, name: &[u8]) -> Option<&Vec<i32>> {
        let a = self.lookup(name);
        if a.is_none_or(|a| a.len() == 1 && is_pua(a[0])) {
            return self.lookup(&normalized_name(name));
        }
        a
    }

    /// `agl_get_unicodes`: the Unicode values of a glyph name (components
    /// joined by `_`, up to a `.`), or `None` (the C's -1).
    pub fn get_unicodes(&self, glyph: &[u8], max_unicodes: usize) -> Option<Vec<i32>> {
        let mut out = Vec::new();
        let end = glyph.iter().position(|&b| b == b'.').unwrap_or(glyph.len());
        let mut p = 0;
        while p < end {
            let delim = glyph[p..].iter().position(|&b| b == b'_').map(|k| p + k);
            if delim == Some(p) {
                return None; // "Invalid glyph name component"
            }
            let delim = match delim {
                Some(d) if d <= end => d,
                _ => end,
            };
            let name = &glyph[p..delim];
            if name_is_unicode(name) {
                if name[1] != b'n' {
                    // uXXXXXXXX
                    if out.len() >= max_unicodes {
                        return None;
                    }
                    out.push(xtol(&name[1..], name.len() - 1));
                } else {
                    let mut q = &name[3..];
                    while !q.is_empty() {
                        if out.len() >= max_unicodes {
                            return None;
                        }
                        out.push(xtol(q, 4));
                        q = &q[q.len().min(4)..];
                    }
                }
            } else {
                let a = self.component(name)?;
                if out.len() + a.len() > max_unicodes {
                    return None;
                }
                out.extend_from_slice(a);
            }
            p = delim + 1;
        }
        Some(out)
    }

    /// `agl_sput_UTF16BE`: the values written for a glyph name and the
    /// number of components that had none.
    pub fn sput(&self, glyph: &[u8]) -> (Vec<i32>, usize) {
        let mut out = Vec::new();
        let mut count = 0;
        let end = glyph.iter().position(|&b| b == b'.').unwrap_or(glyph.len());
        let mut p = 0;
        while p < end {
            let delim = glyph[p..].iter().position(|&b| b == b'_').map(|k| p + k);
            if delim == Some(p) {
                return (out, count + 1); // "Invalid glyph name component"
            }
            let delim = match delim {
                Some(d) if d <= end => d,
                _ => end,
            };
            let name = &glyph[p..delim];
            if name_is_unicode(name) {
                // put_unicode_glyph
                if name[1] != b'n' {
                    out.push(xtol(&name[1..], name.len() - 1));
                } else {
                    let mut q = &name[3..];
                    while !q.is_empty() {
                        out.push(xtol(q, 4));
                        q = &q[q.len().min(4)..];
                    }
                }
            } else {
                match self.component(name) {
                    Some(a) => out.extend_from_slice(a),
                    None => count += 1,
                }
            }
            p = delim + 1;
        }
        (out, count)
    }
}

/// `UC_UTF16BE_encode_char`'s value: one outside 0..=0x10FFFF is written
/// as U+FFFD. (A surrogate value, which the C writes as one UTF-16 unit,
/// has no `char`; it is written as U+FFFD too. No list has one.)
pub fn encode_char(v: i32) -> char {
    if (0..=0x10FFFF).contains(&v) {
        char::from_u32(v as u32).unwrap_or('\u{FFFD}')
    } else {
        '\u{FFFD}'
    }
}

fn is_space(b: u8) -> bool {
    // dvipdfmx's is_space: NUL, TAB, LF, FF, CR, space
    matches!(b, 0 | b'\t' | b'\n' | 0x0C | b'\r' | b' ')
}

/// `parse_ident`'s `valid_chars`.
fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"!\"#$&'*+,-.:;=?@\\^_`|~".contains(&b)
}

/// `IS_PUA`.
fn is_pua(u: i32) -> bool {
    (0xE000..=0xF8FF).contains(&u)
        || (0xF0000..=0xFFFFD).contains(&u)
        || (0x100000..=0x10FFFD).contains(&u)
}

fn is_upper_hex(c: u8) -> bool {
    c.is_ascii_digit() || (b'A'..=b'F').contains(&c)
}

/// `xtol`: `len` characters of upper-case hexadecimal, or -1.
fn xtol(s: &[u8], len: usize) -> i32 {
    let mut v: i32 = 0;
    for k in 0..len {
        let Some(&c) = s.get(k) else {
            return -1; // the string's NUL
        };
        v = v.wrapping_shl(4);
        if c.is_ascii_digit() {
            v += (c - b'0') as i32;
        } else if (b'A'..=b'F').contains(&c) {
            v += (c - b'A' + 10) as i32;
        } else {
            return -1;
        }
    }
    v
}

/// `agl_name_is_unicode`.
pub fn name_is_unicode(g: &[u8]) -> bool {
    let len = g.iter().position(|&b| b == b'.').unwrap_or(g.len());
    if len >= 7 && (len - 3) % 4 == 0 && g.starts_with(b"uni") {
        // "union" is not a Unicode glyph name
        return is_upper_hex(g[3]);
    }
    if (5..=7).contains(&len) && g[0] == b'u' {
        return g[1..len - 1].iter().all(|&c| is_upper_hex(c));
    }
    false
}

/// `skip_capital`: the length of a leading capital (AE, OE, Eth, Thorn, A–Z).
fn skip_capital(s: &[u8]) -> usize {
    if s.len() >= 2 && (s.starts_with(b"AE") || s.starts_with(b"OE")) {
        2
    } else if s.starts_with(b"Eth") {
        3
    } else if s.starts_with(b"Thorn") {
        5
    } else if s.first().is_some_and(|c| c.is_ascii_uppercase()) {
        1
    } else {
        0
    }
}

/// `skip_modifier`: `memcmp(*p, modifiers[i], len)` compares the whole rest
/// (the modifier's NUL included), so only a rest equal to a modifier
/// matches.
fn skip_modifier(s: &[u8]) -> usize {
    MODIFIERS.iter().find(|m| s == **m).map_or(0, |m| m.len())
}

/// `is_smallcap`.
fn is_smallcap(g: &[u8]) -> bool {
    let len = g.len();
    if len < 6 || !g.ends_with(b"small") {
        return false;
    }
    let body = &g[..len - 5];
    let mut rest = body.len();
    let slen = skip_modifier(body);
    if slen == rest {
        return true; // Acutesmall, Gravesmall, ...
    } else if slen > 0 {
        return false;
    }
    let cap = skip_capital(body);
    rest -= cap;
    if rest == 0 {
        return true; // Asmall, AEsmall, ...
    }
    let mut at = cap;
    while rest > 0 {
        let s = skip_modifier(&body[at..]);
        if s == 0 {
            return false;
        }
        at += s;
        rest -= s;
    }
    true
}

/// `agl_guess_name`: the index of the `var_list` key the name ends with.
fn guess_name(g: &[u8]) -> Option<usize> {
    if is_smallcap(g) {
        return Some(0);
    }
    (1..VAR_KEYS.len()).find(|&i| g.len() > VAR_KEYS[i].len() && g.ends_with(VAR_KEYS[i]))
}

/// `agl_normalized_name`'s `name`: before a `.`; a small capital's
/// lower-case base; else without a variant key.
fn normalized_name(g: &[u8]) -> Vec<u8> {
    if let Some(dot) = g.iter().position(|&b| b == b'.') {
        return g[..dot].to_vec();
    }
    if is_smallcap(g) {
        return g[..g.len() - 5]
            .iter()
            .map(|c| c.to_ascii_lowercase())
            .collect();
    }
    match guess_name(g) {
        Some(i) => g[..g.len() - VAR_KEYS[i].len()].to_vec(),
        None => g.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agl() -> Agl {
        let mut a = Agl::default();
        // texglyphlist first: its entries win over glyphlist's
        a.load_list(b"# test\nvisiblespace;2423\nepsilon1;03F5\nffi;FB03\n");
        a.load_list(b"Delta;2206\n");
        a.load_list(b"A;0041\na;0061\nf;0066\ni;0069\nDelta;0394\nepsilon;03B5\nparenleft;0028\nperiodcentered;00B7\nAacute;00C1\naacute;00E1\n");
        a
    }

    #[test]
    fn names() {
        let a = agl();
        assert_eq!(a.get_unicodes(b"visiblespace", 32), Some(vec![0x2423]));
        assert_eq!(a.get_unicodes(b"Delta", 32), Some(vec![0x2206])); // the first list wins
        assert_eq!(a.get_unicodes(b"f_f_i", 32), Some(vec![0x66, 0x66, 0x69]));
        assert_eq!(a.get_unicodes(b"parenleft.v1", 32), Some(vec![0x28])); // up to the `.`
        assert_eq!(a.get_unicodes(b"uni00410042", 32), Some(vec![0x41, 0x42]));
        assert_eq!(a.get_unicodes(b"u1D400", 32), Some(vec![0x1D400]));
        assert_eq!(a.get_unicodes(b"union", 32), None); // not uniXXXX, not in the list
        assert_eq!(a.get_unicodes(b"Asmall", 32), Some(vec![0x61])); // small capital: its base
        assert_eq!(a.get_unicodes(b"Aacutesmall", 32), Some(vec![0xE1]));
        assert_eq!(a.get_unicodes(b"parenleftbig", 32), Some(vec![0x28])); // a variant key
        assert_eq!(a.get_unicodes(b"_a", 32), None);
        assert_eq!(a.get_unicodes(b"nosuch", 32), None);
        assert!(name_is_unicode(b"uni0041.sc"));
        assert!(!name_is_unicode(b"uni0041a"));
    }

    #[test]
    fn sput_counts_failures() {
        let a = agl();
        assert_eq!(a.sput(b"visiblespace"), (vec![0x2423], 0));
        assert_eq!(a.sput(b"epsilon"), (vec![0x3B5], 0));
        assert_eq!(a.sput(b"a_nosuch"), (vec![0x61], 1));
        assert_eq!(a.sput(b"nosuch"), (vec![], 1));
    }

    #[test]
    fn list_lines() {
        let mut a = Agl::default();
        a.load_list(b"  spaced ; 0041 0042\r\nbad line\n;0041\nempty;\n");
        assert_eq!(a.get_unicodes(b"spaced", 32), Some(vec![0x41, 0x42]));
        assert_eq!(a.lookup(b"empty"), None);
    }
}
