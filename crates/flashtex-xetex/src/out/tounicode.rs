//! A native font's `ToUnicode`, as xdvipdfmx writes it: a port of TeX Live
//! 2026's `texk/dvipdfm-x` (GPL-2.0-or-later). `Type0Font_attach_ToUnicode_stream`
//! (`type0.c`) writes none for a font of a standard Adobe character collection
//! (`CIDFont_is_ACCFont`: Adobe-GB1, -CNS1, -Japan1, -Korea1; viewers read the
//! collection), and otherwise `otf_create_ToUnicode_stream` (`tt_cmap.c`):
//!
//! 1. the `cmap` (the first of (3,10), (0,3), (0,4), (0,0), (3,1), (0,1) in
//!    format 4 or 12) read backwards (`create_inverse_cmap4`/`12`): for a
//!    glyph several code points reach, the last one read, which is the
//!    highest; a private-use, presentation-form, CJK-compatibility or
//!    Kangxi-radical code point, or U+00AD, only as a fallback
//!    (`is_PUA_or_presentation`);
//! 2. the glyphs the `GSUB` lookups that some feature of some script lists
//!    (`otl_gsub_add_feat(.., "*", "*", "*")`) make from a glyph with text
//!    (`otl_gsub_add_ToUnicode`, `tt_gsub.c`): single, alternate and
//!    ligature substitutions, in feature order, the first one winning;
//!    lookups reached only from a contextual one are not read;
//! 3. the glyph's name, from the `post` table or (a CFF font not CID-keyed)
//!    its charset, through the Adobe Glyph List
//!    (`add_ToUnicode_via_glyph_name`, [`super::agl`]);
//! 4. the fallback code points of 1.
//!
//! A glyph none of these reaches has no entry; a font with no entry at all
//! has no `ToUnicode`. Each routine names the C it ports.

use super::agl::{encode_char, Agl};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

/// The table `tag` of face `index` of an OpenType file.
pub fn table<'a>(data: &'a [u8], index: u32, tag: &[u8; 4]) -> Option<&'a [u8]> {
    let dir = if data.get(0..4)? == b"ttcf" {
        rd32(data, 12 + 4 * index as usize)? as usize
    } else {
        0
    };
    let n = rd16(data, dir + 4)? as usize;
    for i in 0..n {
        let rec = dir + 12 + 16 * i;
        if data.get(rec..rec + 4)? == tag {
            let off = rd32(data, rec + 8)? as usize;
            let len = rd32(data, rec + 12)? as usize;
            return data.get(off..off + len);
        }
    }
    None
}

fn rd16(d: &[u8], o: usize) -> Option<u16> {
    d.get(o..o + 2).map(|s| u16::from_be_bytes([s[0], s[1]]))
}

fn rd32(d: &[u8], o: usize) -> Option<u32> {
    d.get(o..o + 4)
        .map(|s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
}

/// `CIDFont_is_ACCFont`: an Adobe character collection viewers know, for
/// which xdvipdfmx writes no `ToUnicode`.
pub fn is_acc_font(registry: &[u8], ordering: &[u8]) -> bool {
    registry == b"Adobe" && matches!(ordering, b"GB1" | b"CNS1" | b"Japan1" | b"Korea1")
}

/// `is_PUA_or_presentation`.
fn is_pua_or_presentation(u: u32) -> bool {
    (0x2E80..=0x2EF3).contains(&u)
        || (0x2F00..=0x2FD5).contains(&u)
        || (0xE000..=0xF8FF).contains(&u)
        || (0xFB00..=0xFB4F).contains(&u)
        || (0xF900..=0xFAFF).contains(&u)
        || (0x2F800..=0x2FA1F).contains(&u)
        || (0xF0000..=0xFFFFD).contains(&u)
        || (0x100000..=0x10FFFD).contains(&u)
        || u == 0x00AD
}

/// `UC_is_valid`.
fn uc_is_valid(u: i32) -> bool {
    (0..=0x10FFFF).contains(&u) && !(0xD800..=0xDFFF).contains(&u)
}

/// `cmap_plat_encs[]`.
const PLAT_ENCS: [(u16, u16); 6] = [(3, 10), (0, 3), (0, 4), (0, 0), (3, 1), (0, 1)];

/// `tt_cmap_read` for the first platform/encoding of [`PLAT_ENCS`] whose
/// subtable has format 4 or 12: (format, offset in the `cmap` table).
fn select_cmap(cmap: &[u8]) -> Option<(u16, usize)> {
    let n = rd16(cmap, 2)? as usize;
    for (plat, enc) in PLAT_ENCS {
        for i in 0..n {
            let r = 4 + 8 * i;
            if rd16(cmap, r)? == plat && rd16(cmap, r + 2)? == enc {
                let off = rd32(cmap, r + 4)? as usize;
                let fmt = rd16(cmap, off)?;
                if fmt == 4 || fmt == 12 {
                    return Some((fmt, off));
                }
                break; // tt_cmap_read takes the first matching subtable
            }
        }
    }
    None
}

/// `create_inverse_cmap4` and `create_inverse_cmap12`: `map_base` and
/// `map_sub` (-1: none).
fn inverse_cmap(cmap: &[u8], fmt: u16, off: usize, num_glyphs: usize) -> (Vec<i32>, Vec<i32>) {
    let mut base = vec![-1i32; num_glyphs];
    let mut sub = vec![-1i32; num_glyphs];
    let mut put = |ch: u32, gid: u16| {
        let g = gid as usize;
        if g >= num_glyphs {
            return; // (the C writes past its arrays)
        }
        if is_pua_or_presentation(ch) {
            sub[g] = ch as i32;
        } else {
            base[g] = ch as i32;
        }
    };
    if fmt == 4 {
        let seg_x2 = rd16(cmap, off + 6).unwrap_or(0) as usize;
        let seg_count = seg_x2 / 2;
        let ends = off + 14;
        let starts = ends + seg_x2 + 2;
        let deltas = starts + seg_x2;
        let ranges = deltas + seg_x2;
        let glyphs = ranges + seg_x2;
        for i in 0..seg_count {
            let (Some(c0), Some(c1), Some(delta), Some(ro)) = (
                rd16(cmap, starts + 2 * i),
                rd16(cmap, ends + 2 * i),
                rd16(cmap, deltas + 2 * i),
                rd16(cmap, ranges + 2 * i),
            ) else {
                break;
            };
            let d = (ro / 2).wrapping_sub((seg_count - i) as u16);
            if c1 < c0 {
                continue; // (the C's USHORT loop would not end)
            }
            for j in 0..=(c1 - c0) {
                let ch = c0 + j;
                let gid = if ro == 0 {
                    ch.wrapping_add(delta)
                } else if c0 == 0xFFFF && c1 == 0xFFFF && ro == 0xFFFF {
                    0 // protection against some old broken fonts
                } else {
                    let at = glyphs + 2 * j.wrapping_add(d) as usize;
                    rd16(cmap, at).unwrap_or(0).wrapping_add(delta)
                };
                put(ch as u32, gid);
            }
        }
    } else {
        let groups = rd32(cmap, off + 12).unwrap_or(0) as usize;
        for i in 0..groups {
            let r = off + 16 + 12 * i;
            let (Some(s), Some(e), Some(g0)) =
                (rd32(cmap, r), rd32(cmap, r + 4), rd32(cmap, r + 8))
            else {
                break;
            };
            if e < s || e > 0x10FFFF {
                continue;
            }
            for ch in s..=e {
                put(ch, (g0.wrapping_add(ch - s) & 0xFFFF) as u16);
            }
        }
    }
    (base, sub)
}

/// A coverage table as `clt_read_coverage` reads it.
enum Coverage {
    List(Vec<u16>),
    /// (Start, End, StartCoverageIndex)
    Range(Vec<(u16, u16, u16)>),
}

fn coverage(t: &[u8], o: usize) -> Option<Coverage> {
    let fmt = rd16(t, o)?;
    let n = rd16(t, o + 2)? as usize;
    match fmt {
        1 => Some(Coverage::List(
            (0..n)
                .map(|i| rd16(t, o + 4 + 2 * i))
                .collect::<Option<_>>()?,
        )),
        2 => Some(Coverage::Range(
            (0..n)
                .map(|i| {
                    let r = o + 4 + 6 * i;
                    Some((rd16(t, r)?, rd16(t, r + 2)?, rd16(t, r + 4)?))
                })
                .collect::<Option<_>>()?,
        )),
        _ => None,
    }
}

/// (gid, coverage index) as the `add_ToUnicode_*` loops walk a coverage:
/// a range stops at `num_glyphs`; a list is checked against it only where
/// the C checks it (`list_bound`).
fn walk(cov: &Coverage, num_glyphs: usize, list_bound: bool) -> Vec<(u16, usize)> {
    let mut out = Vec::new();
    match cov {
        Coverage::List(v) => {
            for (idx, &gid) in v.iter().enumerate() {
                if list_bound && gid as usize >= num_glyphs {
                    continue;
                }
                out.push((gid, idx));
            }
        }
        Coverage::Range(v) => {
            for &(s, e, sci) in v {
                let mut gid = s as u32;
                while gid <= e as u32 && (gid as usize) < num_glyphs {
                    out.push((gid as u16, (sci as u32 + gid - s as u32) as usize));
                    gid += 1;
                }
            }
        }
    }
    out
}

/// A `GSUB` subtable xdvipdfmx reads (`otl_gsub_read_single`,
/// `_alternate`, `_ligature`).
enum Subtab {
    Single1(Coverage, i16),
    Single2(Coverage, Vec<u16>),
    Alternate1(Coverage, Vec<Vec<u16>>),
    /// per coverage index: (ligature glyph, the components after the first)
    Ligature1(Coverage, Vec<Vec<(u16, Vec<u16>)>>),
}

fn read_subtab(t: &[u8], ty: u16, so: usize) -> Option<Subtab> {
    let fmt = rd16(t, so)?;
    let cov = coverage(t, so + rd16(t, so + 2)? as usize)?;
    match (ty, fmt) {
        (1, 1) => Some(Subtab::Single1(cov, rd16(t, so + 4)? as i16)),
        (1, 2) => {
            let n = rd16(t, so + 4)? as usize;
            let v = (0..n)
                .map(|i| rd16(t, so + 6 + 2 * i))
                .collect::<Option<_>>()?;
            Some(Subtab::Single2(cov, v))
        }
        (3, 1) => {
            let n = rd16(t, so + 4)? as usize;
            let mut sets = Vec::with_capacity(n);
            for i in 0..n {
                let ao = so + rd16(t, so + 6 + 2 * i)? as usize;
                let k = rd16(t, ao)? as usize;
                sets.push(
                    (0..k)
                        .map(|j| rd16(t, ao + 2 + 2 * j))
                        .collect::<Option<_>>()?,
                );
            }
            Some(Subtab::Alternate1(cov, sets))
        }
        (4, 1) => {
            let n = rd16(t, so + 4)? as usize;
            let mut sets = Vec::with_capacity(n);
            for i in 0..n {
                let lso = so + rd16(t, so + 6 + 2 * i)? as usize;
                let k = rd16(t, lso)? as usize;
                let mut ligs = Vec::with_capacity(k);
                for j in 0..k {
                    let lo = lso + rd16(t, lso + 2 + 2 * j)? as usize;
                    let lig = rd16(t, lo)?;
                    let cc = rd16(t, lo + 2)? as usize;
                    let comps = (0..cc.saturating_sub(1))
                        .map(|c| rd16(t, lo + 4 + 2 * c))
                        .collect::<Option<_>>()?;
                    ligs.push((lig, comps));
                }
                sets.push(ligs);
            }
            Some(Subtab::Ligature1(cov, sets))
        }
        _ => None,
    }
}

/// `otl_gsub_read_feat` for script, language and feature `*`: the
/// subtables of every lookup of every feature that some script's default
/// language system or language system lists, in `FeatureList` order and,
/// within a feature, in its lookup order (a lookup two features list is
/// read twice). Lookups of other types than single (1), alternate (3),
/// ligature (4) and extension (7) to these are skipped.
fn gsub_subtables(gsub: &[u8]) -> Vec<Subtab> {
    let mut out = Vec::new();
    let (Some(sl), Some(fl), Some(ll)) = (rd16(gsub, 4), rd16(gsub, 6), rd16(gsub, 8)) else {
        return out;
    };
    let (sl, fl, ll) = (sl as usize, fl as usize, ll as usize);
    let mut feat_bits = vec![false; 65536];
    let set_langsys = |lo: usize, req_always: bool, bits: &mut [bool]| {
        let req = rd16(gsub, lo + 2).unwrap_or(0xFFFF);
        // dflt: `match("____") && req != 0xFFFF`; a LangSys: `match("____") || req != 0xFFFF`,
        // and "*" matches "____"
        if req_always || req != 0xFFFF {
            bits[req as usize] = true;
        }
        let n = rd16(gsub, lo + 4).unwrap_or(0) as usize;
        for k in 0..n {
            if let Some(f) = rd16(gsub, lo + 6 + 2 * k) {
                bits[f as usize] = true;
            }
        }
    };
    let scripts = rd16(gsub, sl).unwrap_or(0) as usize;
    for i in 0..scripts {
        let Some(so) = rd16(gsub, sl + 2 + 6 * i + 4).map(|o| sl + o as usize) else {
            continue;
        };
        let dflt = rd16(gsub, so).unwrap_or(0);
        if dflt != 0 {
            set_langsys(so + dflt as usize, false, &mut feat_bits);
        }
        let n = rd16(gsub, so + 2).unwrap_or(0) as usize;
        for k in 0..n {
            if let Some(lo) = rd16(gsub, so + 4 + 6 * k + 4) {
                set_langsys(so + lo as usize, true, &mut feat_bits);
            }
        }
    }
    let lookups = rd16(gsub, ll).unwrap_or(0) as usize;
    let features = rd16(gsub, fl).unwrap_or(0) as usize;
    for fi in (0..features).filter(|&f| feat_bits[f]) {
        let Some(ft) = rd16(gsub, fl + 2 + 6 * fi + 4).map(|o| fl + o as usize) else {
            continue;
        };
        let n = rd16(gsub, ft + 2).unwrap_or(0) as usize;
        for k in 0..n {
            let Some(li) = rd16(gsub, ft + 4 + 2 * k).map(|v| v as usize) else {
                continue;
            };
            if li >= lookups {
                continue; // "invalid Lookup index." (an error in xdvipdfmx)
            }
            let Some(lo) = rd16(gsub, ll + 2 + 2 * li).map(|o| ll + o as usize) else {
                continue;
            };
            let ty = rd16(gsub, lo).unwrap_or(0);
            if !matches!(ty, 1 | 3 | 4 | 7) {
                continue; // "Skipping unsupported GSUB subtable"
            }
            let ns = rd16(gsub, lo + 4).unwrap_or(0) as usize;
            for s in 0..ns {
                let Some(so) = rd16(gsub, lo + 6 + 2 * s).map(|o| lo + o as usize) else {
                    continue;
                };
                let st = if ty == 7 {
                    if rd16(gsub, so) != Some(1) {
                        continue;
                    }
                    let (Some(ety), Some(eo)) = (rd16(gsub, so + 2), rd32(gsub, so + 4)) else {
                        continue;
                    };
                    if !matches!(ety, 1 | 3 | 4) {
                        continue;
                    }
                    read_subtab(gsub, ety, so + eo as usize)
                } else {
                    read_subtab(gsub, ty, so)
                };
                if let Some(st) = st {
                    out.push(st);
                }
            }
        }
    }
    out
}

/// `macglyphorder[]` (`tt_post.c`): the standard Macintosh glyph names.
const MAC_GLYPH_ORDER: [&[u8]; 258] = [
    b".notdef",
    b".null",
    b"nonmarkingreturn",
    b"space",
    b"exclam",
    b"quotedbl",
    b"numbersign",
    b"dollar",
    b"percent",
    b"ampersand",
    b"quotesingle",
    b"parenleft",
    b"parenright",
    b"asterisk",
    b"plus",
    b"comma",
    b"hyphen",
    b"period",
    b"slash",
    b"zero",
    b"one",
    b"two",
    b"three",
    b"four",
    b"five",
    b"six",
    b"seven",
    b"eight",
    b"nine",
    b"colon",
    b"semicolon",
    b"less",
    b"equal",
    b"greater",
    b"question",
    b"at",
    b"A",
    b"B",
    b"C",
    b"D",
    b"E",
    b"F",
    b"G",
    b"H",
    b"I",
    b"J",
    b"K",
    b"L",
    b"M",
    b"N",
    b"O",
    b"P",
    b"Q",
    b"R",
    b"S",
    b"T",
    b"U",
    b"V",
    b"W",
    b"X",
    b"Y",
    b"Z",
    b"bracketleft",
    b"backslash",
    b"bracketright",
    b"asciicircum",
    b"underscore",
    b"grave",
    b"a",
    b"b",
    b"c",
    b"d",
    b"e",
    b"f",
    b"g",
    b"h",
    b"i",
    b"j",
    b"k",
    b"l",
    b"m",
    b"n",
    b"o",
    b"p",
    b"q",
    b"r",
    b"s",
    b"t",
    b"u",
    b"v",
    b"w",
    b"x",
    b"y",
    b"z",
    b"braceleft",
    b"bar",
    b"braceright",
    b"asciitilde",
    b"Adieresis",
    b"Aring",
    b"Ccedilla",
    b"Eacute",
    b"Ntilde",
    b"Odieresis",
    b"Udieresis",
    b"aacute",
    b"agrave",
    b"acircumflex",
    b"adieresis",
    b"atilde",
    b"aring",
    b"ccedilla",
    b"eacute",
    b"egrave",
    b"ecircumflex",
    b"edieresis",
    b"iacute",
    b"igrave",
    b"icircumflex",
    b"idieresis",
    b"ntilde",
    b"oacute",
    b"ograve",
    b"ocircumflex",
    b"odieresis",
    b"otilde",
    b"uacute",
    b"ugrave",
    b"ucircumflex",
    b"udieresis",
    b"dagger",
    b"degree",
    b"cent",
    b"sterling",
    b"section",
    b"bullet",
    b"paragraph",
    b"germandbls",
    b"registered",
    b"copyright",
    b"trademark",
    b"acute",
    b"dieresis",
    b"notequal",
    b"AE",
    b"Oslash",
    b"infinity",
    b"plusminus",
    b"lessequal",
    b"greaterequal",
    b"yen",
    b"mu",
    b"partialdiff",
    b"summation",
    b"product",
    b"pi",
    b"integral",
    b"ordfeminine",
    b"ordmasculine",
    b"Omega",
    b"ae",
    b"oslash",
    b"questiondown",
    b"exclamdown",
    b"logicalnot",
    b"radical",
    b"florin",
    b"approxequal",
    b"Delta",
    b"guillemotleft",
    b"guillemotright",
    b"ellipsis",
    b"nonbreakingspace",
    b"Agrave",
    b"Atilde",
    b"Otilde",
    b"OE",
    b"oe",
    b"endash",
    b"emdash",
    b"quotedblleft",
    b"quotedblright",
    b"quoteleft",
    b"quoteright",
    b"divide",
    b"lozenge",
    b"ydieresis",
    b"Ydieresis",
    b"fraction",
    b"currency",
    b"guilsinglleft",
    b"guilsinglright",
    b"fi",
    b"fl",
    b"daggerdbl",
    b"periodcentered",
    b"quotesinglbase",
    b"quotedblbase",
    b"perthousand",
    b"Acircumflex",
    b"Ecircumflex",
    b"Aacute",
    b"Edieresis",
    b"Egrave",
    b"Iacute",
    b"Icircumflex",
    b"Idieresis",
    b"Igrave",
    b"Oacute",
    b"Ocircumflex",
    b"apple",
    b"Ograve",
    b"Uacute",
    b"Ucircumflex",
    b"Ugrave",
    b"dotlessi",
    b"circumflex",
    b"tilde",
    b"macron",
    b"breve",
    b"dotaccent",
    b"ring",
    b"cedilla",
    b"hungarumlaut",
    b"ogonek",
    b"caron",
    b"Lslash",
    b"lslash",
    b"Scaron",
    b"scaron",
    b"Zcaron",
    b"zcaron",
    b"brokenbar",
    b"Eth",
    b"eth",
    b"Yacute",
    b"yacute",
    b"Thorn",
    b"thorn",
    b"minus",
    b"multiply",
    b"onesuperior",
    b"twosuperior",
    b"threesuperior",
    b"onehalf",
    b"onequarter",
    b"threequarters",
    b"franc",
    b"Gbreve",
    b"gbreve",
    b"Idotaccent",
    b"Scedilla",
    b"scedilla",
    b"Cacute",
    b"cacute",
    b"Ccaron",
    b"ccaron",
    b"dcroat",
];

/// `tt_read_post_table`'s names: format 1 the standard ones, format 2 its
/// own; formats 2.5, 3 and 4 none. `None` for a format 2 table xdvipdfmx
/// refuses (an index past its names).
fn post_names(post: &[u8]) -> Option<Vec<Option<Vec<u8>>>> {
    let ver = rd32(post, 0)?;
    if ver == 0x0001_0000 {
        return Some(MAC_GLYPH_ORDER.iter().map(|n| Some(n.to_vec())).collect());
    }
    if ver != 0x0002_0000 {
        return Some(Vec::new());
    }
    let n = rd16(post, 32)? as usize;
    let mut idx = Vec::with_capacity(n);
    let mut maxidx = 257usize;
    for i in 0..n {
        let v = rd16(post, 34 + 2 * i)? as usize;
        if v >= 258 && v > maxidx {
            maxidx = v;
        }
        idx.push(v);
    }
    let count = maxidx - 257;
    let mut names: Vec<Option<Vec<u8>>> = Vec::with_capacity(count);
    let mut q = 34 + 2 * n;
    for _ in 0..count {
        let len = *post.get(q)? as usize;
        q += 1;
        names.push(if len > 0 {
            Some(post.get(q..q + len)?.to_vec())
        } else {
            None
        });
        q += len;
    }
    let mut out = Vec::with_capacity(n);
    for v in idx {
        if v < 258 {
            out.push(Some(MAC_GLYPH_ORDER[v].to_vec()));
        } else if v - 258 < count {
            out.push(names[v - 258].clone());
        } else {
            return None; // "Invalid version 2.0 'post' table"
        }
    }
    Some(out)
}

/// `otf_create_ToUnicode_stream` for face `index` of `data`: the text of
/// each of the `used` codes (glyph ids, or a CID-keyed CFF font's CIDs),
/// or `None` where xdvipdfmx writes no `ToUnicode`.
pub fn otf_to_unicode(
    data: &[u8],
    index: u32,
    used: &BTreeSet<u16>,
    agl: &Agl,
) -> Option<BTreeMap<u16, Vec<char>>> {
    let cmap = table(data, index, b"cmap")?;
    let (fmt, off) = select_cmap(cmap)?;
    let num_glyphs = table(data, index, b"maxp")
        .and_then(|m| rd16(m, 4))
        .unwrap_or(0) as usize;
    let (map_base, map_sub) = inverse_cmap(cmap, fmt, off, num_glyphs);
    // a CFF font: its charset (GIDToCIDMap of a CID-keyed one, names of one
    // that is not)
    let cff = table(data, index, b"CFF ").and_then(|t| flashtex_pdf::cff::CffFont::parse(t).ok());
    let is_cid = cff.as_ref().is_some_and(|c| c.is_cid_keyed());
    let g2c: Vec<u16> = (0..num_glyphs as u32)
        .map(|g| {
            let g = g as u16;
            if is_cid {
                cff.as_ref().and_then(|c| c.charset_entry(g)).unwrap_or(0)
            } else {
                g
            }
        })
        .collect();
    let mut left: BTreeSet<u16> = used.clone();
    let mut out: BTreeMap<u16, Vec<char>> = BTreeMap::new();
    let mut count = 0usize;
    let text = |v: &[i32]| v.iter().map(|&u| encode_char(u)).collect::<Vec<char>>();

    // the cmap's own code points
    for (gid, &cid) in g2c.iter().enumerate() {
        if left.contains(&cid) && uc_is_valid(map_base[gid]) {
            out.insert(cid, text(&[map_base[gid]]));
            left.remove(&cid);
            count += 1;
        }
    }
    // otl_gsub_add_ToUnicode
    if let Some(gsub) = table(data, index, b"GSUB") {
        // add_glyph_if_valid
        let add_glyph = |gid: u16,
                         gid_sub: u16,
                         left: &mut BTreeSet<u16>,
                         out: &mut BTreeMap<u16, Vec<char>>| {
            let (g, gs) = (gid as usize, gid_sub as usize);
            if gs >= num_glyphs || g >= num_glyphs {
                return 0;
            }
            let cid_sub = g2c[gs];
            if !left.contains(&cid_sub) {
                return 0;
            }
            let ch = if uc_is_valid(map_base[g]) {
                map_base[g]
            } else if uc_is_valid(map_sub[g]) {
                map_sub[g]
            } else {
                return 0;
            };
            out.insert(cid_sub, text(&[ch]));
            left.remove(&cid_sub);
            1
        };
        for st in gsub_subtables(gsub) {
            match &st {
                Subtab::Single1(cov, delta) => {
                    for (gid, _) in walk(cov, num_glyphs, false) {
                        count +=
                            add_glyph(gid, gid.wrapping_add(*delta as u16), &mut left, &mut out);
                    }
                }
                Subtab::Single2(cov, subs) => {
                    for (gid, idx) in walk(cov, num_glyphs, false) {
                        if let Some(&gs) = subs.get(idx) {
                            count += add_glyph(gid, gs, &mut left, &mut out);
                        }
                    }
                }
                Subtab::Alternate1(cov, sets) => {
                    for (gid, idx) in walk(cov, num_glyphs, true) {
                        if let Some(set) = sets.get(idx) {
                            for &ga in set {
                                count += add_glyph(gid, ga, &mut left, &mut out);
                            }
                        }
                    }
                }
                Subtab::Ligature1(cov, sets) => {
                    for (gid, idx) in walk(cov, num_glyphs, true) {
                        let Some(set) = sets.get(idx) else {
                            continue;
                        };
                        for (lig, comps) in set {
                            let l = *lig as usize;
                            if l >= num_glyphs {
                                continue;
                            }
                            let cid = g2c[l];
                            if !left.contains(&cid) {
                                continue;
                            }
                            let mut ucv = Vec::with_capacity(comps.len() + 1);
                            let mut fail = 0;
                            for &g in std::iter::once(&gid).chain(comps.iter()) {
                                let g = g as usize;
                                if g >= num_glyphs {
                                    fail += 1;
                                    continue;
                                }
                                let ch = if uc_is_valid(map_base[g]) {
                                    map_base[g]
                                } else {
                                    map_sub[g]
                                };
                                if !uc_is_valid(ch) {
                                    fail += 1;
                                }
                                ucv.push(ch);
                            }
                            if fail == 0 {
                                out.insert(cid, text(&ucv));
                                left.remove(&cid);
                                // (add_ToUnicode_ligature returns 0: not counted)
                            }
                        }
                    }
                }
            }
        }
    }
    // add_ToUnicode_via_glyph_name: the post table's names, else (a CFF
    // font not CID-keyed) the charset's
    let post = table(data, index, b"post").and_then(post_names);
    let names_cff = cff.as_ref().filter(|_| !is_cid);
    if post.is_some() || names_cff.is_some() {
        for (gid, &cid) in g2c.iter().enumerate() {
            if !left.contains(&cid) {
                continue;
            }
            let name: Option<Vec<u8>> = post
                .as_ref()
                .and_then(|p| p.get(gid).cloned().flatten())
                .or_else(|| {
                    names_cff
                        .and_then(|c| c.glyph_name(gid as u16))
                        .map(String::into_bytes)
                });
            let Some(name) = name else {
                continue;
            };
            if let Some(us) = agl.get_unicodes(&name, 32).filter(|u| !u.is_empty()) {
                out.insert(cid, text(&us));
                left.remove(&cid);
                count += 1;
            }
        }
    }
    // finally, the private-use and presentation forms
    for (gid, &cid) in g2c.iter().enumerate() {
        if left.contains(&cid) && uc_is_valid(map_sub[gid]) {
            out.insert(cid, text(&[map_sub[gid]]));
            left.remove(&cid);
            count += 1;
        }
    }
    if count < 1 {
        None
    } else {
        Some(out)
    }
}

/// `pdf_create_ToUnicode_CMap` (`pdfencoding.c`): a Type 1 font's text by
/// its encoding's glyph names (`agl_sput_UTF16BE`); none at all when a used
/// glyph's name has no text, or none has any.
pub fn encoding_to_unicode(
    names: &BTreeMap<u16, Vec<u8>>,
    agl: &Agl,
) -> Option<BTreeMap<u16, Vec<char>>> {
    let mut out = BTreeMap::new();
    let mut total_fail = 0;
    for (&code, name) in names {
        let (v, fail) = agl.sput(name);
        if v.is_empty() && fail > 0 {
            total_fail += 1;
        } else {
            out.insert(code, v.iter().map(|&u| encode_char(u)).collect());
        }
    }
    if out.is_empty() || total_fail > 0 {
        None
    } else {
        Some(out)
    }
}

/// The `cmap` read backwards (step 1 above): each glyph's code point, the
/// fallback ones only where it has no other.
pub fn cmap_inverse(data: &[u8], index: u32) -> BTreeMap<u16, char> {
    let mut out = BTreeMap::new();
    let Some(cmap) = table(data, index, b"cmap") else {
        return out;
    };
    let Some((fmt, off)) = select_cmap(cmap) else {
        return out;
    };
    let num_glyphs = table(data, index, b"maxp")
        .and_then(|m| rd16(m, 4))
        .unwrap_or(0) as usize;
    let (base, sub) = inverse_cmap(cmap, fmt, off, num_glyphs);
    for g in 0..num_glyphs {
        let v = if uc_is_valid(base[g]) {
            base[g]
        } else {
            sub[g]
        };
        if uc_is_valid(v) {
            out.insert(g as u16, encode_char(v));
        }
    }
    out
}

/// `CMap_create_stream` (`cmap_write.c`) for a `ToUnicode` CMap named
/// `name` (`CMap_set_name`'s, `-UTF16` already appended): the CMap file
/// text, codes 2 bytes (`Identity-H`) or 1 (a Type 1 font's encoding).
/// `None` when no code has text (an empty value is an invalid mapping
/// `CMap_add_bfchar` drops, and a CMap with none is not written).
pub fn cmap_stream(name: &[u8], map: &BTreeMap<u16, Vec<char>>, two_byte: bool) -> Option<String> {
    // the values, UTF-16BE (`UC_UTF16BE_encode_char`), by code
    let mut vals: BTreeMap<u16, Vec<u8>> = BTreeMap::new();
    for (&code, chars) in map {
        let mut v = Vec::with_capacity(2 * chars.len());
        for ch in chars {
            let mut buf = [0u16; 2];
            for unit in ch.encode_utf16(&mut buf) {
                v.extend_from_slice(&unit.to_be_bytes());
            }
        }
        if !v.is_empty() {
            vals.insert(if two_byte { code } else { code & 0xFF }, v);
        }
    }
    if vals.is_empty() {
        return None; // "Invalid CMap": no mapTbl
    }
    let max_in = if two_byte { 2 } else { 1 };
    let max_out = vals.values().map(Vec::len).max().unwrap_or(0);
    let mut w = CmapWriter {
        out: String::from("/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n"),
        wbuf: String::new(),
        lim: (40960 + 16usize).saturating_sub(2 * (max_in + max_out)),
        count: 0,
    };
    w.out.push_str("/CMapName ");
    cmap_name(name, &mut w.out);
    w.out.push_str(" def\n/CMapType 2 def\n");
    w.out.push_str(
        "/CIDSystemInfo <<\n  /Registry (Adobe)\n  /Ordering (UCS)\n  /Supplement 0\n>> def\n",
    );
    w.out.push_str(if two_byte {
        "1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n"
    } else {
        "1 begincodespacerange\n<00> <FF>\nendcodespacerange\n"
    });
    if two_byte {
        // the top table: a second-level table for each high byte in use
        for hi in 0..256u16 {
            let tbl: Vec<Option<&Vec<u8>>> =
                (0..256u16).map(|lo| vals.get(&(hi << 8 | lo))).collect();
            if tbl.iter().any(Option::is_some) {
                w.write_map(&tbl, Some(hi as u8));
            }
            w.flush_if_full();
        }
    } else {
        let tbl: Vec<Option<&Vec<u8>>> = (0..256u16).map(|c| vals.get(&c)).collect();
        w.write_map(&tbl, None);
    }
    if w.count > 0 {
        w.flush_bfchar();
    }
    w.out
        .push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
    Some(w.out)
}

/// `cmap_write.c`'s state: the stream so far, the pending `bfchar` lines
/// (`wbuf`, flushed at its limit `limptr`) and their count.
struct CmapWriter {
    out: String,
    wbuf: String,
    lim: usize,
    count: usize,
}

impl CmapWriter {
    fn flush_bfchar(&mut self) {
        let _ = writeln!(self.out, "{} beginbfchar", self.count);
        self.out.push_str(&self.wbuf);
        self.out.push_str("endbfchar\n");
        self.wbuf.clear();
        self.count = 0;
    }

    /// `write_map`'s "Flush if necessary".
    fn flush_if_full(&mut self) {
        if self.count >= 100 || self.wbuf.len() >= self.lim {
            self.flush_bfchar();
        }
    }

    /// `write_map` for one table of 256 codes (`prefix`: the high byte of
    /// a 2-byte code): a run of three or more codes whose values differ by
    /// one in their last byte (`block_count`) is a `bfrange`, written
    /// after the table's `bfchar`s; the others are `bfchar`s, flushed in
    /// blocks of 100 that may span tables.
    fn write_map(&mut self, tbl: &[Option<&Vec<u8>>], prefix: Option<u8>) {
        let mut blocks: Vec<(usize, usize)> = Vec::new();
        let pre = prefix.map(|p| format!("{p:02X}")).unwrap_or_default();
        let mut c = 0usize;
        while c < 256 {
            if let Some(v) = tbl[c] {
                let n = block_count(tbl, c);
                if n >= 2 {
                    blocks.push((c, n));
                    c += n;
                } else {
                    let _ = write!(self.wbuf, "<{pre}{c:02X}> <");
                    push_hex(&mut self.wbuf, v);
                    self.wbuf.push_str(">\n");
                    self.count += 1;
                }
            }
            self.flush_if_full();
            c += 1;
        }
        if !blocks.is_empty() {
            if self.count > 0 {
                self.flush_bfchar();
            }
            let _ = writeln!(self.out, "{} beginbfrange", blocks.len());
            for &(c, n) in &blocks {
                let _ = write!(self.out, "<{pre}{c:02X}> <{pre}{:02X}> <", (c + n) & 0xFF);
                if let Some(v) = tbl[c] {
                    push_hex(&mut self.out, v);
                }
                self.out.push_str(">\n");
            }
            self.out.push_str("endbfrange\n");
        }
    }
}

/// `block_count`: how many codes after `c` continue its run.
fn block_count(tbl: &[Option<&Vec<u8>>], c: usize) -> usize {
    let Some(first) = tbl[c] else { return 0 };
    let n = first.len() - 1;
    let mut count = 0;
    for d in c + 1..256 {
        let (Some(prev), Some(cur)) = (tbl[d - 1], tbl[d]) else {
            break;
        };
        if prev.len() != cur.len() {
            break;
        }
        if prev[..n] == cur[..n] && prev[n] < 255 && prev[n] + 1 == cur[n] {
            count += 1;
        } else {
            break;
        }
    }
    count
}

/// `sputx`, byte by byte.
fn push_hex(s: &mut String, bytes: &[u8]) {
    for b in bytes {
        let _ = write!(s, "{b:02X}");
    }
}

/// `cmap_write.c`'s `write_name`.
fn cmap_name(name: &[u8], s: &mut String) {
    s.push('/');
    for &b in name {
        if !(b'!'..=b'~').contains(&b) || b"#()/<>[]{}%".contains(&b) {
            let _ = write!(s, "#{b:02X}");
        } else {
            s.push(b as char);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presentation_forms_are_fallbacks() {
        assert!(is_pua_or_presentation(0x2F47)); // KANGXI RADICAL SUN
        assert!(is_pua_or_presentation(0xFB01)); // fi
        assert!(is_pua_or_presentation(0xE000));
        assert!(is_pua_or_presentation(0x00AD));
        assert!(!is_pua_or_presentation(0xFE8D)); // Arabic forms-B: not in the list
        assert!(!is_pua_or_presentation(0x65E5));
        assert!(is_acc_font(b"Adobe", b"Japan1"));
        assert!(!is_acc_font(b"Adobe", b"Identity"));
    }

    /// A format 4 `cmap` with two code points for one glyph, one of them a
    /// Kangxi radical: the ideograph is the text.
    #[test]
    fn inverse_cmap4() {
        // segments [0x2F47..0x2F47 -> 5], [0x65E5..0x65E5 -> 5], [0xFFFF]
        let starts = [0x2F47u16, 0x65E5, 0xFFFF];
        let ends = [0x2F47u16, 0x65E5, 0xFFFF];
        let deltas = [5u16.wrapping_sub(0x2F47), 5u16.wrapping_sub(0x65E5), 1];
        let mut t = vec![0u8; 4];
        t[2..4].copy_from_slice(&1u16.to_be_bytes());
        t.extend_from_slice(&[0, 3, 0, 1, 0, 0, 0, 12]); // (3,1) at 12
        let mut sub = Vec::new();
        for v in [4u16, 0, 0, 6, 0, 0, 0] {
            sub.extend_from_slice(&v.to_be_bytes());
        }
        for e in ends {
            sub.extend_from_slice(&e.to_be_bytes());
        }
        sub.extend_from_slice(&0u16.to_be_bytes());
        for s in starts {
            sub.extend_from_slice(&s.to_be_bytes());
        }
        for d in deltas {
            sub.extend_from_slice(&d.to_be_bytes());
        }
        for _ in 0..3 {
            sub.extend_from_slice(&0u16.to_be_bytes());
        }
        t.extend_from_slice(&sub);
        let (fmt, off) = select_cmap(&t).unwrap();
        assert_eq!(fmt, 4);
        let (base, subm) = inverse_cmap(&t, fmt, off, 8);
        assert_eq!(base[5], 0x65E5);
        assert_eq!(subm[5], 0x2F47);
        // the 0xFFFF segment maps to glyph 0 (idDelta 1): a code point the
        // writer then refuses for want of a used glyph 0, or writes
        assert_eq!(base[0], 0xFFFF);
    }

    /// A `ToUnicode` CMap xdvipdfmx 20260317 wrote (TeX Live 2026 `xetex`,
    /// streams decoded by `qpdf --qdf`) read back to its name, code width
    /// and mappings.
    fn read_cmap(text: &str) -> (Vec<u8>, bool, BTreeMap<u16, Vec<char>>) {
        let name = text
            .split("/CMapName /")
            .nth(1)
            .and_then(|s| s.split(' ').next())
            .unwrap()
            .as_bytes()
            .to_vec();
        let two_byte = text.contains("<0000> <FFFF>");
        let hex = |s: &str| -> Vec<u8> {
            (0..s.len() / 2)
                .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
                .collect()
        };
        let chars = |v: &[u8]| -> Vec<char> {
            let units: Vec<u16> = v
                .chunks(2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect();
            char::decode_utf16(units).map(Result::unwrap).collect()
        };
        let mut map = BTreeMap::new();
        let mut mode = "";
        for line in text.lines() {
            if line.ends_with("beginbfchar") || line.ends_with("beginbfrange") {
                mode = if line.ends_with("char") {
                    "char"
                } else {
                    "range"
                };
                continue;
            }
            if line.starts_with("end") {
                mode = "";
                continue;
            }
            let f: Vec<&str> = line
                .split(' ')
                .map(|s| s.trim_matches(['<', '>']))
                .collect();
            match mode {
                "char" => {
                    map.insert(u16::from_str_radix(f[0], 16).unwrap(), chars(&hex(f[1])));
                }
                "range" => {
                    let lo = u16::from_str_radix(f[0], 16).unwrap();
                    let hi = u16::from_str_radix(f[1], 16).unwrap();
                    let mut v = hex(f[2]);
                    for code in lo..=hi {
                        map.insert(code, chars(&v));
                        *v.last_mut().unwrap() += 1;
                    }
                }
                _ => {}
            }
        }
        (name, two_byte, map)
    }

    /// `CMap_create_stream`'s text, byte for byte: its header, runs of three
    /// or more as `bfrange`s after each 256-code table's `bfchar`s, `bfchar`
    /// blocks of at most 100 carried across tables. Oracle streams: a Type 1
    /// font's 1-byte codes (cmr10: ligatures U+FB00..FB03 as one range),
    /// ArialMT's codes 32..299 (100-entry blocks), STIX Two Math's
    /// mathematical alphanumerics (surrogate pairs in ranges), and Latin
    /// Modern Roman's "ffi" (ff + i, a GSUB ligature's text).
    #[test]
    fn cmap_stream_is_cmap_write_text() {
        for text in [
            include_str!("testdata/tounicode-cmr10.cmap"),
            include_str!("testdata/tounicode-arial.cmap"),
            include_str!("testdata/tounicode-stixtwomath.cmap"),
            include_str!("testdata/tounicode-lmroman-ligature.cmap"),
        ] {
            let (name, two_byte, map) = read_cmap(text);
            assert!(!map.is_empty());
            assert_eq!(cmap_stream(&name, &map, two_byte).as_deref(), Some(text));
        }
        // an empty text is no mapping; none at all, no CMap
        let mut m = BTreeMap::new();
        m.insert(5u16, Vec::new());
        assert_eq!(cmap_stream(b"X-UTF16", &m, false), None);
        m.insert(6u16, vec!['a']);
        let s = cmap_stream(b"X Y-UTF16", &m, false).unwrap();
        assert!(s.contains("/CMapName /X#20Y-UTF16 def\n"));
        assert!(s.contains("1 beginbfchar\n<06> <0061>\nendbfchar\n"));
    }

    #[test]
    fn type1_encoding_is_all_or_nothing() {
        let mut agl = Agl::default();
        agl.load_list(b"visiblespace;2423\nA;0041\n");
        let mut names = BTreeMap::new();
        names.insert(32u16, b"visiblespace".to_vec());
        names.insert(65u16, b"A".to_vec());
        let m = encoding_to_unicode(&names, &agl).unwrap();
        assert_eq!(m[&32], vec!['\u{2423}']);
        names.insert(66u16, b"nosuchglyph".to_vec());
        assert_eq!(encoding_to_unicode(&names, &agl), None);
    }
}
