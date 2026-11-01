//! Subsets of CID-keyed CFF fonts (CJK OpenType fonts such as Source Han,
//! Noto CJK, HaranoAji): the glyphs a document draws, kept with their
//! CIDs, so that the content stream (which writes CIDs) and `/W` stay as
//! they are. `crates/pdf`'s CFF subsetter takes name-keyed fonts only.
//!
//! The subset (Adobe Technical Note #5176, *The Compact Font Format*):
//!
//! * glyphs renumbered densely (`.notdef` first), the charset mapping each
//!   new glyph to its original CID, `FDSelect` to its original Font DICT;
//! * every Font DICT and Private DICT kept (they are small), the global
//!   and each Font DICT's local subroutines kept at their indices but
//!   emptied when no kept glyph reaches them, so that no `callsubr` or
//!   `callgsubr` operand has to change;
//! * the Top DICT, Name and String INDEXes as they were, with the offsets
//!   rewritten (as 5-byte integers).
//!
//! Which subroutines a glyph reaches is found by reading its Type 2
//! charstring (Adobe Technical Note #5177): the number pushed before a
//! `callsubr`/`callgsubr` (plus the bias) names the subroutine, and the
//! stem hints counted from `hstem`/`vstem` and their `hm` forms give the
//! length of each `hintmask`/`cntrmask`.

use std::collections::{BTreeMap, BTreeSet};

type R<T> = Result<T, String>;

fn u8_at(d: &[u8], o: usize) -> R<u8> {
    d.get(o)
        .copied()
        .ok_or_else(|| format!("CFF truncated at {o}"))
}

fn u16_at(d: &[u8], o: usize) -> R<u16> {
    Ok(u16::from_be_bytes([u8_at(d, o)?, u8_at(d, o + 1)?]))
}

/// An INDEX: its items' byte ranges, and where it ends.
struct Index {
    items: Vec<(usize, usize)>,
    end: usize,
}

fn index(d: &[u8], at: usize) -> R<Index> {
    let count = u16_at(d, at)? as usize;
    if count == 0 {
        return Ok(Index {
            items: vec![],
            end: at + 2,
        });
    }
    let os = u8_at(d, at + 2)? as usize;
    if !(1..=4).contains(&os) {
        return Err(format!("CFF INDEX offset size {os}"));
    }
    let off = |i: usize| -> R<usize> {
        let p = at + 3 + i * os;
        let mut v = 0usize;
        for k in 0..os {
            v = (v << 8) | u8_at(d, p + k)? as usize;
        }
        Ok(v)
    };
    let data = at + 3 + (count + 1) * os - 1;
    let mut items = Vec::with_capacity(count);
    for i in 0..count {
        let (s, e) = (data + off(i)?, data + off(i + 1)?);
        if s > e || e > d.len() {
            return Err("CFF INDEX item outside the table".into());
        }
        items.push((s, e));
    }
    let end = data + off(count)?;
    Ok(Index { items, end })
}

fn write_index(items: &[&[u8]]) -> Vec<u8> {
    let mut out = (items.len() as u16).to_be_bytes().to_vec();
    if items.is_empty() {
        return out;
    }
    let total: usize = items.iter().map(|i| i.len()).sum::<usize>() + 1;
    let os = match total {
        0..=0xFF => 1,
        0x100..=0xFFFF => 2,
        0x1_0000..=0xFF_FFFF => 3,
        _ => 4,
    };
    out.push(os as u8);
    let mut o = 1usize;
    let put = |out: &mut Vec<u8>, v: usize| {
        for k in (0..os).rev() {
            out.push((v >> (8 * k)) as u8);
        }
    };
    put(&mut out, o);
    for i in items {
        o += i.len();
        put(&mut out, o);
    }
    for i in items {
        out.extend_from_slice(i);
    }
    out
}

/// A DICT: (operator, operand bytes as written, operand values).
type Dict = Vec<(u16, Vec<u8>, Vec<f64>)>;

fn dict(d: &[u8]) -> R<Dict> {
    let mut out = Vec::new();
    let mut i = 0;
    let mut st = i;
    let mut vals = Vec::new();
    while i < d.len() {
        let b = d[i];
        match b {
            0..=21 => {
                let (op, n) = if b == 12 {
                    (1200 + u8_at(d, i + 1)? as u16, 2)
                } else {
                    (b as u16, 1)
                };
                out.push((op, d[st..i].to_vec(), std::mem::take(&mut vals)));
                i += n;
                st = i;
            }
            28 => {
                vals.push(i16::from_be_bytes([u8_at(d, i + 1)?, u8_at(d, i + 2)?]) as f64);
                i += 3;
            }
            29 => {
                vals.push(i32::from_be_bytes([
                    u8_at(d, i + 1)?,
                    u8_at(d, i + 2)?,
                    u8_at(d, i + 3)?,
                    u8_at(d, i + 4)?,
                ]) as f64);
                i += 5;
            }
            30 => {
                // a real: nibbles to 0xf
                i += 1;
                loop {
                    let n = u8_at(d, i)?;
                    i += 1;
                    if n & 0x0F == 0x0F || n >> 4 == 0x0F {
                        break;
                    }
                }
                vals.push(0.0);
            }
            32..=246 => {
                vals.push(b as f64 - 139.0);
                i += 1;
            }
            247..=250 => {
                vals.push(((b as f64 - 247.0) * 256.0) + u8_at(d, i + 1)? as f64 + 108.0);
                i += 2;
            }
            251..=254 => {
                vals.push(-((b as f64 - 251.0) * 256.0) - u8_at(d, i + 1)? as f64 - 108.0);
                i += 2;
            }
            _ => return Err(format!("CFF DICT byte {b}")),
        }
    }
    Ok(out)
}

fn get(d: &Dict, op: u16) -> Option<&[f64]> {
    d.iter().find(|e| e.0 == op).map(|e| e.2.as_slice())
}

fn int5(v: usize) -> Vec<u8> {
    let mut o = vec![29];
    o.extend_from_slice(&(v as i32).to_be_bytes());
    o
}

fn put_op(out: &mut Vec<u8>, op: u16) {
    if op >= 1200 {
        out.push(12);
        out.push((op - 1200) as u8);
    } else {
        out.push(op as u8);
    }
}

/// `d` written back, with the operands of `replace`'s operators replaced
/// (already encoded) and the operators of `drop` left out.
fn write_dict(d: &Dict, replace: &BTreeMap<u16, Vec<u8>>, drop: &[u16]) -> Vec<u8> {
    let mut out = Vec::new();
    for (op, raw, _) in d {
        if drop.contains(op) {
            continue;
        }
        match replace.get(op) {
            Some(v) => out.extend_from_slice(v),
            None => out.extend_from_slice(raw),
        }
        put_op(&mut out, *op);
    }
    out
}

fn bias(n: usize) -> i64 {
    if n < 1240 {
        107
    } else if n < 33900 {
        1131
    } else {
        32768
    }
}

/// Which subroutines a charstring reaches.
struct Marks<'a> {
    d: &'a [u8],
    gsubrs: &'a [(usize, usize)],
    lsubrs: &'a [(usize, usize)],
    used_g: BTreeSet<usize>,
    used_l: BTreeSet<usize>,
    stems: usize,
    stack: Vec<f64>,
    depth: u32,
}

impl Marks<'_> {
    /// Read charstring bytes `s..e`; true at `endchar`.
    fn run(&mut self, s: usize, e: usize) -> R<bool> {
        self.depth += 1;
        if self.depth > 64 {
            return Err("CFF subroutines nested too deep".into());
        }
        let d = self.d;
        let mut i = s;
        while i < e {
            let b = d[i];
            match b {
                28 => {
                    let v = i16::from_be_bytes([u8_at(d, i + 1)?, u8_at(d, i + 2)?]);
                    self.stack.push(v as f64);
                    i += 3;
                }
                32..=246 => {
                    self.stack.push(b as f64 - 139.0);
                    i += 1;
                }
                247..=250 => {
                    self.stack
                        .push((b as f64 - 247.0) * 256.0 + u8_at(d, i + 1)? as f64 + 108.0);
                    i += 2;
                }
                251..=254 => {
                    self.stack
                        .push(-(b as f64 - 251.0) * 256.0 - u8_at(d, i + 1)? as f64 - 108.0);
                    i += 2;
                }
                255 => {
                    self.stack.push(0.0);
                    i += 5;
                }
                1 | 3 | 18 | 23 => {
                    // hstem, vstem, hstemhm, vstemhm
                    self.stems += self.stack.len() / 2;
                    self.stack.clear();
                    i += 1;
                }
                19 | 20 => {
                    // hintmask, cntrmask: an implicit vstem first
                    self.stems += self.stack.len() / 2;
                    self.stack.clear();
                    i += 1 + self.stems.div_ceil(8);
                }
                10 | 29 => {
                    let local = b == 10;
                    let list = if local { self.lsubrs } else { self.gsubrs };
                    let n = self.stack.pop().ok_or("callsubr without an index")? as i64
                        + bias(list.len());
                    let n = usize::try_from(n).map_err(|_| "subroutine index below 0")?;
                    let &(ss, se) = list.get(n).ok_or("subroutine index out of range")?;
                    if local {
                        self.used_l.insert(n);
                    } else {
                        self.used_g.insert(n);
                    }
                    if self.run(ss, se)? {
                        self.depth -= 1;
                        return Ok(true);
                    }
                    i += 1;
                }
                11 => {
                    self.depth -= 1;
                    return Ok(false);
                }
                14 => {
                    self.depth -= 1;
                    return Ok(true);
                }
                12 => {
                    // two-byte operators: arithmetic keeps a result, the
                    // flex operators clear the stack; neither calls
                    self.stack.clear();
                    i += 2;
                }
                _ => {
                    self.stack.clear();
                    i += 1;
                }
            }
        }
        self.depth -= 1;
        Ok(false)
    }
}

/// A Font DICT, its Private DICT and its local subroutines.
struct Fd {
    dict: Dict,
    private: Dict,
    subrs: Option<Index>,
}

/// A parsed CID-keyed CFF table.
struct Font {
    hdr: usize,
    names: Index,
    tops: Index,
    strings: Index,
    gsubrs: Index,
    top: Dict,
    charstrings: Index,
    /// By glyph: its CID and its Font DICT.
    cid: Vec<u16>,
    fd: Vec<u8>,
    fds: Vec<Fd>,
}

fn parse(cff: &[u8]) -> R<Font> {
    let hdr = u8_at(cff, 2)? as usize;
    let names = index(cff, hdr)?;
    let tops = index(cff, names.end)?;
    let strings = index(cff, tops.end)?;
    let gsubrs = index(cff, strings.end)?;
    let &(ts, te) = tops.items.first().ok_or("CFF without a Top DICT")?;
    let top = dict(&cff[ts..te])?;
    if get(&top, 1230).is_none() {
        return Err("not a CID-keyed CFF".into());
    }
    let off = |op: u16| -> R<usize> {
        get(&top, op)
            .and_then(|v| v.first())
            .map(|&v| v as usize)
            .ok_or_else(|| format!("CFF Top DICT without operator {op}"))
    };
    let charstrings = index(cff, off(17)?)?;
    let n = charstrings.items.len();
    let fdarray = index(cff, off(1236)?)?;
    // charset: gid -> CID
    let mut cid = vec![0u16; n];
    let cs_at = off(15)?;
    let fmt = u8_at(cff, cs_at)?;
    let mut p = cs_at + 1;
    let mut g = 1usize;
    while g < n {
        match fmt {
            0 => {
                cid[g] = u16_at(cff, p)?;
                p += 2;
                g += 1;
            }
            1 | 2 => {
                let first = u16_at(cff, p)?;
                let left = if fmt == 1 {
                    u8_at(cff, p + 2)? as usize
                } else {
                    u16_at(cff, p + 2)? as usize
                };
                p += if fmt == 1 { 3 } else { 4 };
                for k in 0..=left {
                    if g >= n {
                        break;
                    }
                    cid[g] = first.wrapping_add(k as u16);
                    g += 1;
                }
            }
            _ => return Err(format!("CFF charset format {fmt}")),
        }
    }
    // FDSelect: gid -> FD
    let mut fd = vec![0u8; n];
    let fs_at = off(1237)?;
    match u8_at(cff, fs_at)? {
        0 => {
            for (k, f) in fd.iter_mut().enumerate() {
                *f = u8_at(cff, fs_at + 1 + k)?;
            }
        }
        3 => {
            let nr = u16_at(cff, fs_at + 1)? as usize;
            for r in 0..nr {
                let at = fs_at + 3 + 3 * r;
                let (first, f) = (u16_at(cff, at)? as usize, u8_at(cff, at + 2)?);
                let next = u16_at(cff, at + 3)? as usize;
                for slot in fd.iter_mut().take(next.min(n)).skip(first) {
                    *slot = f;
                }
            }
        }
        f => return Err(format!("CFF FDSelect format {f}")),
    }
    // each Font DICT's Private DICT and local subroutines
    let mut fds = Vec::new();
    for &(s, e) in &fdarray.items {
        let fdict = dict(&cff[s..e])?;
        let (size, at) = match get(&fdict, 18) {
            Some([size, at]) => (*size as usize, *at as usize),
            _ => (0, 0),
        };
        let private = dict(
            cff.get(at..at + size)
                .ok_or("Private DICT outside the table")?,
        )?;
        let subrs = match get(&private, 19).and_then(|v| v.first()) {
            Some(&o) => Some(index(cff, at + o as usize)?),
            None => None,
        };
        fds.push(Fd {
            dict: fdict,
            private,
            subrs,
        });
    }
    Ok(Font {
        hdr,
        names,
        tops,
        strings,
        gsubrs,
        top,
        charstrings,
        cid,
        fd,
        fds,
    })
}

/// The font's `ROS` (registry, ordering, supplement), when its strings
/// are the font's own (SIDs from 391), e.g. `Adobe`, `Japan1`, 7: the
/// `CIDSystemInfo` of its CIDFont, through whose ordering viewers read its
/// CIDs (xdvipdfmx writes it, measured).
pub fn ros(cff: &[u8]) -> Option<(Vec<u8>, Vec<u8>, i64)> {
    let f = parse(cff).ok()?;
    let r = get(&f.top, 1230)?;
    let s = |sid: f64| -> Option<Vec<u8>> {
        let k = (sid as usize).checked_sub(391)?;
        let &(a, b) = f.strings.items.get(k)?;
        Some(cff[a..b].to_vec())
    };
    Some((s(*r.first()?)?, s(*r.get(1)?)?, *r.get(2)? as i64))
}

/// The global and local subroutines glyph `g` reaches.
fn reached(cff: &[u8], f: &Font, g: usize) -> R<(BTreeSet<usize>, BTreeSet<usize>)> {
    let lsubrs: &[(usize, usize)] = f
        .fds
        .get(f.fd[g] as usize)
        .and_then(|x| x.subrs.as_ref())
        .map_or(&[], |s| &s.items);
    let mut m = Marks {
        d: cff,
        gsubrs: &f.gsubrs.items,
        lsubrs,
        used_g: BTreeSet::new(),
        used_l: BTreeSet::new(),
        stems: 0,
        stack: Vec::new(),
        depth: 0,
    };
    let (s, e) = *f.charstrings.items.get(g).ok_or("glyph out of range")?;
    m.run(s, e)?;
    Ok((m.used_g, m.used_l))
}

/// The subset of CID-keyed CFF table `cff` holding glyphs `gids` (and
/// `.notdef`), each keeping its CID.
pub fn subset(cff: &[u8], gids: &BTreeSet<u16>) -> R<Vec<u8>> {
    let Font {
        hdr,
        names,
        tops,
        strings,
        gsubrs,
        top,
        charstrings,
        cid,
        fd,
        fds,
    } = parse(cff)?;
    let n = charstrings.items.len();
    // the glyphs kept, and the subroutines they reach
    let mut keep: Vec<usize> = vec![0];
    keep.extend(gids.iter().map(|&g| g as usize).filter(|&g| g > 0 && g < n));
    keep.dedup();
    let mut used_g = BTreeSet::new();
    let mut used_l: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); fds.len()];
    let font = Font {
        hdr,
        names,
        tops,
        strings,
        gsubrs,
        top,
        charstrings,
        cid,
        fd,
        fds,
    };
    for &g in &keep {
        let (ug, ul) = reached(cff, &font, g)?;
        used_g.extend(ug);
        if let Some(u) = used_l.get_mut(font.fd[g] as usize) {
            u.extend(ul);
        }
    }
    let Font {
        hdr,
        names,
        tops,
        strings,
        gsubrs,
        top,
        charstrings,
        cid,
        fd,
        fds,
    } = font;
    let blank_unused = |idx: &Index, used: &BTreeSet<usize>| -> Vec<u8> {
        let items: Vec<&[u8]> = idx
            .items
            .iter()
            .enumerate()
            .map(|(k, &(s, e))| {
                if used.contains(&k) {
                    &cff[s..e]
                } else {
                    &[][..]
                }
            })
            .collect();
        write_index(&items)
    };
    // the parts after the Top DICT, at offsets from the table's start
    let name_idx = &cff[hdr..names.end];
    let string_idx = &cff[tops.end..strings.end];
    let gsub = blank_unused(&gsubrs, &used_g);
    let mut charset = vec![0u8];
    for &g in &keep[1..] {
        charset.extend_from_slice(&cid[g].to_be_bytes());
    }
    let mut fdselect = vec![0u8];
    fdselect.extend(keep.iter().map(|&g| fd[g]));
    let cs_items: Vec<&[u8]> = keep
        .iter()
        .map(|&g| {
            let (s, e) = charstrings.items[g];
            &cff[s..e]
        })
        .collect();
    let cs_idx = write_index(&cs_items);
    // Private DICTs and local subroutines, each subroutine INDEX right after
    // its Private DICT
    let mut privs: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    for (k, f) in fds.iter().enumerate() {
        let subrs = f.subrs.as_ref().map(|s| blank_unused(s, &used_l[k]));
        let mut rep = BTreeMap::new();
        let mut p = write_dict(&f.private, &BTreeMap::new(), &[19]);
        if subrs.is_some() {
            // the Subrs operand: the Private DICT's own size, as a 5-byte int
            rep.insert(19u16, int5(0));
            let mut with = write_dict(&f.private, &rep, &[]);
            let size = with.len();
            rep.insert(19u16, int5(size));
            with = write_dict(&f.private, &rep, &[]);
            p = with;
        }
        privs.push((p, subrs.unwrap_or_default()));
    }
    // the Top DICT with placeholder offsets fixes its size
    let top_with = |charset_at: usize, fdsel_at: usize, cs_at: usize, fda_at: usize| -> Vec<u8> {
        let mut rep = BTreeMap::new();
        rep.insert(15u16, int5(charset_at));
        rep.insert(1237u16, int5(fdsel_at));
        rep.insert(17u16, int5(cs_at));
        rep.insert(1236u16, int5(fda_at));
        write_dict(&top, &rep, &[16, 18])
    };
    let top_len = write_index(&[&top_with(0, 0, 0, 0)]).len();
    let mut at = 4 + name_idx.len() + top_len + string_idx.len() + gsub.len();
    let charset_at = at;
    at += charset.len();
    let fdsel_at = at;
    at += fdselect.len();
    let cs_at2 = at;
    at += cs_idx.len();
    let fda_at = at;
    // the FDArray, whose Private offsets follow it: its size does not
    // depend on them (5-byte ints)
    let fd_dicts = |priv_at: &[usize]| -> Vec<Vec<u8>> {
        fds.iter()
            .enumerate()
            .map(|(k, f)| {
                let mut rep = BTreeMap::new();
                let mut v = int5(privs[k].0.len());
                v.extend(int5(priv_at[k]));
                rep.insert(18u16, v);
                write_dict(&f.dict, &rep, &[])
            })
            .collect()
    };
    let probe = fd_dicts(&vec![0; fds.len()]);
    let probe_refs: Vec<&[u8]> = probe.iter().map(Vec::as_slice).collect();
    at += write_index(&probe_refs).len();
    let mut priv_at = Vec::new();
    for (p, s) in &privs {
        priv_at.push(at);
        at += p.len() + s.len();
    }
    let fdas = fd_dicts(&priv_at);
    let fda_refs: Vec<&[u8]> = fdas.iter().map(Vec::as_slice).collect();
    let mut out = Vec::with_capacity(at);
    out.extend_from_slice(&[1, 0, 4, u8_at(cff, 3)?]);
    out.extend_from_slice(name_idx);
    out.extend(write_index(&[&top_with(
        charset_at, fdsel_at, cs_at2, fda_at,
    )]));
    out.extend_from_slice(string_idx);
    out.extend(gsub);
    out.extend(charset);
    out.extend(fdselect);
    out.extend(cs_idx);
    out.extend(write_index(&fda_refs));
    for (p, s) in privs {
        out.extend(p);
        out.extend(s);
    }
    debug_assert_eq!(out.len(), at);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TeX Live's HaranoAji Mincho (CID-keyed, Adobe-Japan1), if present.
    fn haranoaji() -> Option<Vec<u8>> {
        let p = std::process::Command::new("kpsewhich")
            .arg("HaranoAjiMincho-Regular.otf")
            .output()
            .ok()?;
        let p = String::from_utf8_lossy(&p.stdout).trim().to_string();
        (!p.is_empty()).then(|| std::fs::read(p).ok()).flatten()
    }

    /// The subset keeps each glyph's CID and outline (every subroutine its
    /// charstring reaches), and is small.
    #[test]
    fn a_subset_keeps_cids_and_outlines() {
        let Some(data) = haranoaji() else {
            eprintln!("skipped: no HaranoAjiMincho-Regular.otf (TeX Live)");
            return;
        };
        let font = flashtex_pdf::truetype::TrueTypeFont::parse(data).unwrap();
        let table = font.cff_table().unwrap();
        let src = flashtex_pdf::cff::CffFont::parse(table).unwrap();
        let gids: BTreeSet<u16> = "漢字日文、。あア"
            .chars()
            .map(|c| font.glyph_id(c).unwrap())
            .collect();
        let out = subset(table, &gids).unwrap();
        assert!(out.len() < 60_000, "{} bytes", out.len());
        let sub = flashtex_pdf::cff::CffFont::parse(&out).unwrap();
        assert!(sub.is_cid_keyed());
        assert_eq!(sub.glyph_count() as usize, gids.len() + 1);
        let (a, b) = (parse(table).unwrap(), parse(&out).unwrap());
        let body = |d: &[u8], i: &Index, k: usize| d[i.items[k].0..i.items[k].1].to_vec();
        for (new, &old) in (1u16..).zip(&gids) {
            let (new, old) = (new as usize, old as usize);
            assert_eq!(sub.charset_entry(new as u16), src.charset_entry(old as u16));
            assert_eq!(b.cid[new], a.cid[old]);
            // the same charstring, and every subroutine it reaches the same
            assert_eq!(
                body(&out, &b.charstrings, new),
                body(table, &a.charstrings, old)
            );
            let (ga, la) = reached(table, &a, old).unwrap();
            let (gb, lb) = reached(&out, &b, new).unwrap();
            assert_eq!((&ga, &la), (&gb, &lb), "glyph {old}");
            for k in ga {
                assert_eq!(body(&out, &b.gsubrs, k), body(table, &a.gsubrs, k));
            }
            let (fa, fb) = (a.fd[old] as usize, b.fd[new] as usize);
            assert_eq!(fa, fb);
            for k in la {
                let (sa, sb) = (
                    a.fds[fa].subrs.as_ref().unwrap(),
                    b.fds[fb].subrs.as_ref().unwrap(),
                );
                assert!(!body(&out, sb, k).is_empty());
                assert_eq!(body(&out, sb, k), body(table, sa, k));
            }
        }
    }
}
