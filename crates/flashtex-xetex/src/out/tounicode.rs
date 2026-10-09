//! The text of a native font's glyphs for `ToUnicode`: the font's `cmap`
//! read backwards, then what its `GSUB` substitutions say about glyphs the
//! `cmap` does not reach, as xdvipdfmx derives it (measured on TeX Gyre and
//! Latin Modern: a ligature's text is its components' `cmap` text, so the
//! `ffi` ligature built from `ff` and `i` is "\u{FB00}i", and `ff` is "ff";
//! a glyph a single or alternate substitution makes is the text of the
//! glyph it replaces).

use std::collections::{BTreeMap, BTreeSet};

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

/// The font's Unicode `cmap` backwards: each glyph's lowest code point
/// (formats 4 and 12 of the (3, 10), (0, *) or (3, 1) subtable).
pub fn cmap_reverse(data: &[u8], index: u32) -> BTreeMap<u16, Vec<char>> {
    let mut out: BTreeMap<u16, Vec<char>> = BTreeMap::new();
    let Some(t) = table(data, index, b"cmap") else {
        return out;
    };
    let n = rd16(t, 2).unwrap_or(0) as usize;
    let mut best: Option<(u8, usize)> = None;
    for i in 0..n {
        let r = 4 + 8 * i;
        let (Some(pid), Some(eid), Some(off)) = (rd16(t, r), rd16(t, r + 2), rd32(t, r + 4)) else {
            continue;
        };
        let off = off as usize;
        let fmt = rd16(t, off).unwrap_or(0);
        let rank = match (pid, eid, fmt) {
            (3, 10, 12) | (0, 4, 12) | (0, 6, 12) => 3,
            (0, _, 12) => 3,
            (0, _, 4) | (3, 1, 4) => 2,
            _ => continue,
        };
        if best.is_none_or(|(b, _)| rank > b) {
            best = Some((rank, off));
        }
    }
    let Some((_, off)) = best else {
        return out;
    };
    let mut put = |cp: u32, g: u16| {
        if g == 0 {
            return;
        }
        if let Some(c) = char::from_u32(cp) {
            out.entry(g).or_insert_with(|| vec![c]);
        }
    };
    match rd16(t, off) {
        Some(4) => {
            let segs = rd16(t, off + 6).unwrap_or(0) as usize / 2;
            let ends = off + 14;
            let starts = ends + 2 * segs + 2;
            let deltas = starts + 2 * segs;
            let ranges = deltas + 2 * segs;
            for s in 0..segs {
                let (Some(e), Some(st), Some(d), Some(ro)) = (
                    rd16(t, ends + 2 * s),
                    rd16(t, starts + 2 * s),
                    rd16(t, deltas + 2 * s),
                    rd16(t, ranges + 2 * s),
                ) else {
                    break;
                };
                if st > e || st == 0xFFFF {
                    continue;
                }
                for c in st..=e {
                    let g = if ro == 0 {
                        c.wrapping_add(d)
                    } else {
                        let at = ranges + 2 * s + ro as usize + 2 * (c - st) as usize;
                        match rd16(t, at) {
                            Some(0) | None => 0,
                            Some(g) => g.wrapping_add(d),
                        }
                    };
                    put(c as u32, g);
                }
            }
        }
        Some(12) => {
            let groups = rd32(t, off + 12).unwrap_or(0) as usize;
            for k in 0..groups {
                let r = off + 16 + 12 * k;
                let (Some(s), Some(e), Some(g0)) = (rd32(t, r), rd32(t, r + 4), rd32(t, r + 8))
                else {
                    break;
                };
                if e < s || e - s > 0x10FFFF {
                    continue;
                }
                for c in s..=e {
                    put(c, (g0 + (c - s)) as u16);
                }
            }
        }
        _ => {}
    }
    out
}

/// The `MATH` table's size variants: (base glyph, variant glyph), vertical
/// and horizontal.
fn math_variants(m: &[u8]) -> Vec<(u16, u16)> {
    let mut out = Vec::new();
    let Some(mv) = rd16(m, 8).map(|o| o as usize).filter(|&o| o != 0) else {
        return out;
    };
    let (Some(vc), Some(hc), Some(vn), Some(hn)) = (
        rd16(m, mv + 2),
        rd16(m, mv + 4),
        rd16(m, mv + 6),
        rd16(m, mv + 8),
    ) else {
        return out;
    };
    let vcov = if vc != 0 {
        coverage(m, mv + vc as usize)
    } else {
        Vec::new()
    };
    let hcov = if hc != 0 {
        coverage(m, mv + hc as usize)
    } else {
        Vec::new()
    };
    let lists = [
        (vcov, 0usize, vn as usize),
        (hcov, vn as usize, hn as usize),
    ];
    for (cov, first, n) in lists {
        for (i, &base) in cov.iter().enumerate().take(n) {
            let Some(co) = rd16(m, mv + 10 + 2 * (first + i)).map(|o| mv + o as usize) else {
                continue;
            };
            let k = rd16(m, co + 2).unwrap_or(0) as usize;
            for j in 0..k {
                if let Some(g) = rd16(m, co + 4 + 4 * j) {
                    if g != base {
                        out.push((base, g));
                    }
                }
            }
            // the parts of its assembly (extenders, ends)
            if let Some(ao) = rd16(m, co).filter(|&o| o != 0).map(|o| co + o as usize) {
                let parts = rd16(m, ao + 4).unwrap_or(0) as usize;
                for j in 0..parts {
                    if let Some(g) = rd16(m, ao + 6 + 10 * j) {
                        if g != base {
                            out.push((base, g));
                        }
                    }
                }
            }
        }
    }
    out
}

/// The glyphs of a coverage table, in coverage order.
fn coverage(t: &[u8], o: usize) -> Vec<u16> {
    let mut out = Vec::new();
    match rd16(t, o) {
        Some(1) => {
            let n = rd16(t, o + 2).unwrap_or(0) as usize;
            for i in 0..n {
                if let Some(g) = rd16(t, o + 4 + 2 * i) {
                    out.push(g);
                }
            }
        }
        Some(2) => {
            let n = rd16(t, o + 2).unwrap_or(0) as usize;
            for i in 0..n {
                let r = o + 4 + 6 * i;
                let (Some(s), Some(e)) = (rd16(t, r), rd16(t, r + 2)) else {
                    break;
                };
                for g in s..=e.max(s) {
                    out.push(g);
                }
            }
        }
        _ => {}
    }
    out
}

/// The substitutions of a `GSUB` table: (input glyphs, output glyph), for
/// single, alternate and ligature lookups (extension ones included).
fn substitutions(gsub: &[u8]) -> Vec<(Vec<u16>, u16)> {
    let mut out = Vec::new();
    let Some(ll) = rd16(gsub, 8).map(|o| o as usize) else {
        return out;
    };
    let n = rd16(gsub, ll).unwrap_or(0) as usize;
    for i in 0..n {
        let Some(lo) = rd16(gsub, ll + 2 + 2 * i).map(|o| ll + o as usize) else {
            continue;
        };
        let ty = rd16(gsub, lo).unwrap_or(0);
        let ns = rd16(gsub, lo + 4).unwrap_or(0) as usize;
        for k in 0..ns {
            let Some(so) = rd16(gsub, lo + 6 + 2 * k).map(|o| lo + o as usize) else {
                continue;
            };
            let (ty, so) = if ty == 7 {
                let et = rd16(gsub, so + 2).unwrap_or(0);
                let eo = rd32(gsub, so + 4).unwrap_or(0) as usize;
                (et, so + eo)
            } else {
                (ty, so)
            };
            subtable(gsub, ty, so, &mut out);
        }
    }
    out
}

fn subtable(t: &[u8], ty: u16, so: usize, out: &mut Vec<(Vec<u16>, u16)>) {
    let fmt = rd16(t, so).unwrap_or(0);
    let cov = match rd16(t, so + 2) {
        Some(c) => coverage(t, so + c as usize),
        None => return,
    };
    match (ty, fmt) {
        (1, 1) => {
            let delta = rd16(t, so + 4).unwrap_or(0);
            for g in cov {
                out.push((vec![g], g.wrapping_add(delta)));
            }
        }
        (1, 2) => {
            for (i, g) in cov.into_iter().enumerate() {
                if let Some(s) = rd16(t, so + 6 + 2 * i) {
                    out.push((vec![g], s));
                }
            }
        }
        (3, 1) => {
            for (i, g) in cov.into_iter().enumerate() {
                let Some(ao) = rd16(t, so + 6 + 2 * i).map(|o| so + o as usize) else {
                    continue;
                };
                let n = rd16(t, ao).unwrap_or(0) as usize;
                for k in 0..n {
                    if let Some(a) = rd16(t, ao + 2 + 2 * k) {
                        out.push((vec![g], a));
                    }
                }
            }
        }
        (4, 1) => {
            for (i, g) in cov.into_iter().enumerate() {
                let Some(lso) = rd16(t, so + 6 + 2 * i).map(|o| so + o as usize) else {
                    continue;
                };
                let n = rd16(t, lso).unwrap_or(0) as usize;
                for k in 0..n {
                    let Some(lo) = rd16(t, lso + 2 + 2 * k).map(|o| lso + o as usize) else {
                        continue;
                    };
                    let (Some(lig), Some(cc)) = (rd16(t, lo), rd16(t, lo + 2)) else {
                        continue;
                    };
                    let mut comps = vec![g];
                    for c in 1..cc as usize {
                        if let Some(x) = rd16(t, lo + 4 + 2 * (c - 1)) {
                            comps.push(x);
                        }
                    }
                    out.push((comps, lig));
                }
            }
        }
        _ => {}
    }
}

/// The text of each of `used` glyphs: `cmap` gives `base` (glyph → its
/// text); a ligature's text is its components' `base` text; a glyph only a
/// single or alternate substitution reaches has the text of the glyph it
/// replaces (repeatedly, through chains).
pub fn glyph_text(
    data: &[u8],
    index: u32,
    base: &BTreeMap<u16, Vec<char>>,
    used: &BTreeSet<u16>,
) -> BTreeMap<u16, Vec<char>> {
    let mut out: BTreeMap<u16, Vec<char>> = BTreeMap::new();
    for &g in used {
        if let Some(t) = base.get(&g) {
            out.insert(g, t.clone());
        }
    }
    let Some(gsub) = table(data, index, b"GSUB") else {
        return out;
    };
    let subs = substitutions(gsub);
    // ligatures: from the components' cmap text, over the cmap's own
    let mut lig_done = BTreeSet::new();
    for (comps, lig) in &subs {
        if comps.len() < 2 || !used.contains(lig) || lig_done.contains(lig) {
            continue;
        }
        let presentation =
            |t: &Vec<char>| t.len() == 1 && ('\u{FB00}'..='\u{FB06}').contains(&t[0]);
        if base.get(lig).is_some_and(|t| !presentation(t)) {
            continue;
        }
        let text: Option<Vec<char>> = comps
            .iter()
            .map(|c| base.get(c).cloned())
            .collect::<Option<Vec<_>>>()
            .map(|v| v.concat());
        if let Some(t) = text {
            out.insert(*lig, t);
            lig_done.insert(*lig);
        }
    }
    // `.notdef`, when it is drawn: U+FFFF (measured)
    if used.contains(&0) {
        out.insert(0, vec!['\u{FFFF}']);
    }
    let mut known: BTreeMap<u16, Vec<char>> = base.clone();
    // single and alternate substitutions, to a fixed point
    for _ in 0..4 {
        let mut changed = false;
        for (comps, to) in &subs {
            if comps.len() != 1 || known.contains_key(to) {
                continue;
            }
            if let Some(t) = known.get(&comps[0]).cloned() {
                known.insert(*to, t);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // then the variants and assembly parts of the `MATH` table: their
    // base glyph's text
    if let Some(math) = table(data, index, b"MATH") {
        for (b, v) in math_variants(math) {
            if !known.contains_key(&v) {
                if let Some(t) = known.get(&b).cloned() {
                    known.insert(v, t);
                }
            }
        }
    }
    for &g in used {
        if let (std::collections::btree_map::Entry::Vacant(e), Some(t)) =
            (out.entry(g), known.get(&g))
        {
            e.insert(t.clone());
        }
    }
    out
}
