//! The font map's entries and its two trees (`mapfile.c`'s `fm_entry`
//! list, `tfm_tree` and `ps_tree`), kept compact (lane MEM-MODES).
//!
//! `pdftex.map` has about 46,000 entries. Parsed into one `FmEntry` each
//! (five heap-allocated names) and two `BTreeMap`s keyed by copies of the
//! names, it took 17-24 MB of every engine's heap, although a document uses
//! a few dozen entries. Here a map file read into an empty map is a
//! [`Base`]: its parse in a compact byte form (the disk cache's, see
//! `mapfile::disk`), mapped from the cache file when there is one (so its
//! pages are the file's, not the process's memory) and otherwise held in
//! one buffer, with an offset per entry and per tree element. An entry is
//! decoded the first time it is read, once per process; a tree is searched
//! in place. What a run changes afterwards (`\pdfmapline`, `\pdfmapfile`:
//! entries added, replaced or deleted) is kept apart, over the base.
//!
//! Every read answers exactly what the plain vector and trees would: the
//! base is the parse those were built from, and a change is looked up
//! before the base. The persisted encoding ([`FmTable::enc_parts`]) is the
//! plain form's, byte for byte.

use super::mapfile::FmEntry;
use super::shared::Shared;
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

/// `ps_tree`'s key: PostScript name, slant and extend.
pub type PsKey = (Vec<u8>, i32, i32);

/// The bytes of a [`Base`]: a mapped cache file or a buffer.
enum Bytes {
    Mapped(crate::os::MappedFile, usize),
    Owned(Vec<u8>),
}

impl Bytes {
    fn get(&self) -> &[u8] {
        match self {
            Bytes::Mapped(m, from) => &m.bytes()[*from..],
            Bytes::Owned(v) => v,
        }
    }
}

const NONE: u32 = u32::MAX;

/// A parsed map file, read only: see the module documentation.
pub struct Base {
    bytes: Bytes,
    /// Each entry's record, or [`NONE`] for a deleted one.
    entry_at: Box<[u32]>,
    /// Each decoded entry, once read.
    cells: Box<[OnceLock<Box<Option<FmEntry>>>]>,
    /// `tfm_tree`'s elements, in key order: each record's offset.
    tfm_at: Box<[u32]>,
    /// `ps_tree`'s elements, in key order.
    ps_at: Box<[u32]>,
}

static NO_ENTRY: Option<FmEntry> = None;

/// A cursor over the compact form.
struct Cur<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Cur<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let s = self.b.get(self.pos..self.pos.checked_add(n)?)?;
        self.pos += n;
        Some(s)
    }
    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take(2)?.try_into().ok()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn i32(&mut self) -> Option<i32> {
        Some(i32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn bytes(&mut self) -> Option<&'a [u8]> {
        let n = self.u32()? as usize;
        self.take(n)
    }
    fn opt(&mut self) -> Option<Option<&'a [u8]>> {
        match self.u8()? {
            0 => Some(None),
            1 => Some(Some(self.bytes()?)),
            _ => None,
        }
    }
    /// A count no larger than what is left could hold.
    fn count(&mut self) -> Option<usize> {
        let n = self.u32()? as usize;
        (n <= self.b.len() - self.pos).then_some(n)
    }
    /// An entry's record (after its presence byte).
    fn entry(&mut self) -> Option<FmEntry> {
        Some(FmEntry {
            tfm_name: self.bytes()?.to_vec(),
            ps_name: self.opt()?.map(<[u8]>::to_vec),
            fd_flags: self.i32()?,
            slant: self.i32()?,
            extend: self.i32()?,
            encname: self.opt()?.map(<[u8]>::to_vec),
            ff_name: self.opt()?.map(<[u8]>::to_vec),
            typ: self.u16()?,
            pid: self.u16()? as i16,
            eid: self.u16()? as i16,
            links: self.u16()?,
            subfont: match self.u8()? {
                0 => None,
                1 => {
                    let n = self.count()?;
                    Some((0..n).map(|_| self.i32()).collect::<Option<Vec<i32>>>()?)
                }
                _ => return None,
            },
        })
    }
}

fn put_bytes(w: &mut Vec<u8>, b: &[u8]) {
    w.extend_from_slice(&(b.len() as u32).to_le_bytes());
    w.extend_from_slice(b);
}

fn put_opt(w: &mut Vec<u8>, b: &Option<Vec<u8>>) {
    match b {
        None => w.push(0),
        Some(b) => {
            w.push(1);
            put_bytes(w, b);
        }
    }
}

/// The compact form of a parse: the entries, then `tfm_tree`'s and
/// `ps_tree`'s elements in key order (the disk cache's body).
pub fn encode<'a>(
    fms: impl ExactSizeIterator<Item = Option<&'a FmEntry>>,
    tfm: impl ExactSizeIterator<Item = (&'a [u8], usize)>,
    ps: impl ExactSizeIterator<Item = (&'a PsKey, usize)>,
    w: &mut Vec<u8>,
) {
    w.extend_from_slice(&(fms.len() as u32).to_le_bytes());
    for e in fms {
        let Some(e) = e else {
            w.push(0);
            continue;
        };
        w.push(1);
        put_bytes(w, &e.tfm_name);
        put_opt(w, &e.ps_name);
        w.extend_from_slice(&e.fd_flags.to_le_bytes());
        w.extend_from_slice(&e.slant.to_le_bytes());
        w.extend_from_slice(&e.extend.to_le_bytes());
        put_opt(w, &e.encname);
        put_opt(w, &e.ff_name);
        w.extend_from_slice(&e.typ.to_le_bytes());
        w.extend_from_slice(&e.pid.to_le_bytes());
        w.extend_from_slice(&e.eid.to_le_bytes());
        w.extend_from_slice(&e.links.to_le_bytes());
        match &e.subfont {
            None => w.push(0),
            Some(v) => {
                w.push(1);
                w.extend_from_slice(&(v.len() as u32).to_le_bytes());
                for x in v {
                    w.extend_from_slice(&x.to_le_bytes());
                }
            }
        }
    }
    w.extend_from_slice(&(tfm.len() as u32).to_le_bytes());
    for (k, v) in tfm {
        put_bytes(w, k);
        w.extend_from_slice(&(v as u32).to_le_bytes());
    }
    w.extend_from_slice(&(ps.len() as u32).to_le_bytes());
    for ((name, slant, extend), v) in ps {
        put_bytes(w, name);
        w.extend_from_slice(&slant.to_le_bytes());
        w.extend_from_slice(&extend.to_le_bytes());
        w.extend_from_slice(&(v as u32).to_le_bytes());
    }
}

impl Base {
    /// A base over a mapped cache file whose compact form starts at `from`;
    /// `None` if the bytes are not one (truncated, unsorted, out of range).
    pub fn mapped(m: crate::os::MappedFile, from: usize) -> Option<Base> {
        (from <= m.bytes().len()).then_some(())?;
        Base::new(Bytes::Mapped(m, from))
    }

    /// A base over a buffer holding the compact form.
    pub fn owned(v: Vec<u8>) -> Option<Base> {
        Base::new(Bytes::Owned(v))
    }

    /// Index the compact form, checking all of it: every record is read
    /// through once here, so a later decode cannot fail.
    fn new(bytes: Bytes) -> Option<Base> {
        let b = bytes.get();
        if b.len() > u32::MAX as usize {
            return None;
        }
        let mut c = Cur { b, pos: 0 };
        let n = c.count()?;
        let mut entry_at = Vec::with_capacity(n);
        for _ in 0..n {
            if c.u8()? == 0 {
                entry_at.push(NONE);
                continue;
            }
            entry_at.push(c.pos as u32);
            c.entry()?;
        }
        let tree = |c: &mut Cur, ps: bool| -> Option<Vec<u32>> {
            let m = c.count()?;
            let mut at = Vec::with_capacity(m);
            let mut last: Option<(&[u8], i32, i32)> = None;
            for _ in 0..m {
                at.push(c.pos as u32);
                let k = c.bytes()?;
                let (s, e) = if ps { (c.i32()?, c.i32()?) } else { (0, 0) };
                if (c.u32()? as usize) >= n {
                    return None;
                }
                if last.is_some_and(|l| l >= (k, s, e)) {
                    return None;
                }
                last = Some((k, s, e));
            }
            Some(at)
        };
        let tfm_at = tree(&mut c, false)?;
        let ps_at = tree(&mut c, true)?;
        if c.pos != b.len() {
            return None;
        }
        let cells = (0..n).map(|_| OnceLock::new()).collect();
        Some(Base {
            bytes,
            entry_at: entry_at.into(),
            cells,
            tfm_at: tfm_at.into(),
            ps_at: ps_at.into(),
        })
    }

    fn len(&self) -> usize {
        self.entry_at.len()
    }

    /// Entry `i`, decoded once.
    fn entry(&self, i: usize) -> &Option<FmEntry> {
        let at = self.entry_at[i];
        if at == NONE {
            return &NO_ENTRY;
        }
        self.cells[i].get_or_init(|| {
            let mut c = Cur {
                b: self.bytes.get(),
                pos: at as usize,
            };
            Box::new(c.entry())
        })
    }

    /// Entry `i`, decoded afresh (not kept: a reading of every entry,
    /// for an encoding, must not keep them all).
    fn decode(&self, i: usize) -> Option<FmEntry> {
        let at = self.entry_at[i];
        if at == NONE {
            return None;
        }
        if let Some(e) = self.cells[i].get() {
            return (**e).clone();
        }
        let mut c = Cur {
            b: self.bytes.get(),
            pos: at as usize,
        };
        c.entry()
    }

    /// Tree element `at`: its key and its entry.
    fn element(&self, at: u32, ps: bool) -> ((&[u8], i32, i32), usize) {
        let mut c = Cur {
            b: self.bytes.get(),
            pos: at as usize,
        };
        // checked in `new`
        let k = c.bytes().unwrap_or_default();
        let (s, e) = if ps {
            (c.i32().unwrap_or(0), c.i32().unwrap_or(0))
        } else {
            (0, 0)
        };
        ((k, s, e), c.u32().unwrap_or(0) as usize)
    }

    fn find(&self, ps: bool, key: (&[u8], i32, i32)) -> Option<usize> {
        let at = if ps { &self.ps_at } else { &self.tfm_at };
        at.binary_search_by(|&a| self.element(a, ps).0.cmp(&key))
            .ok()
            .map(|i| self.element(at[i], ps).1)
    }

    fn tfm_elements(&self) -> impl Iterator<Item = (&[u8], usize)> + '_ {
        self.tfm_at.iter().map(|&a| {
            let ((k, _, _), v) = self.element(a, false);
            (k, v)
        })
    }

    fn ps_elements(&self) -> impl Iterator<Item = ((&[u8], i32, i32), usize)> + '_ {
        self.ps_at.iter().map(|&a| self.element(a, true))
    }
}

/// The font map's entries and trees: a [`Base`] (or none) and the changes
/// over it. Cloning shares everything (a checkpoint's copy).
#[derive(Clone, Default)]
pub struct FmTable {
    base: Option<Arc<Base>>,
    /// Base entries changed or deleted since.
    changed: Shared<BTreeMap<usize, Option<FmEntry>>>,
    /// Entries after the base's.
    added: Shared<Vec<Option<FmEntry>>>,
    /// `tfm_tree` over the base's: a key's entry, or `None` once removed.
    tfm: Shared<BTreeMap<Vec<u8>, Option<usize>>>,
    /// `ps_tree` over the base's.
    ps: Shared<BTreeMap<PsKey, Option<usize>>>,
}

impl std::ops::Index<usize> for FmTable {
    type Output = Option<FmEntry>;
    fn index(&self, i: usize) -> &Option<FmEntry> {
        let nb = self.base_len();
        if i < nb {
            if let Some(e) = self.changed.get(&i) {
                return e;
            }
            return self.base.as_ref().unwrap().entry(i);
        }
        &self.added[i - nb]
    }
}

impl FmTable {
    /// The table of a map file parsed into an empty map.
    pub fn from_base(base: Arc<Base>) -> FmTable {
        FmTable {
            base: Some(base),
            ..FmTable::default()
        }
    }

    fn base_len(&self) -> usize {
        self.base.as_ref().map_or(0, |b| b.len())
    }

    /// The number of entries (the next entry's index).
    pub fn len(&self) -> usize {
        self.base_len() + self.added.len()
    }

    /// No entries and empty trees (an empty map).
    pub fn is_empty(&self) -> bool {
        self.base.is_none() && self.added.is_empty() && self.tfm.is_empty() && self.ps.is_empty()
    }

    pub fn push(&mut self, e: Option<FmEntry>) {
        self.added.push(e);
    }

    /// Entry `i`, to change it.
    pub fn get_mut(&mut self, i: usize) -> &mut Option<FmEntry> {
        let nb = self.base_len();
        if i < nb {
            if !self.changed.contains_key(&i) {
                let e = self.base.as_ref().unwrap().entry(i).clone();
                self.changed.insert(i, e);
            }
            return self.changed.get_mut(&i).unwrap();
        }
        &mut self.added[i - nb]
    }

    pub fn tfm_get(&self, k: &[u8]) -> Option<usize> {
        match self.tfm.get(k) {
            Some(v) => *v,
            None => self.base.as_ref()?.find(false, (k, 0, 0)),
        }
    }

    pub fn tfm_insert(&mut self, k: Vec<u8>, v: usize) {
        self.tfm.insert(k, Some(v));
    }

    pub fn tfm_remove(&mut self, k: &[u8]) {
        if self
            .base
            .as_ref()
            .is_some_and(|b| b.find(false, (k, 0, 0)).is_some())
        {
            self.tfm.insert(k.to_vec(), None);
        } else {
            self.tfm.remove(k);
        }
    }

    pub fn ps_get(&self, k: &PsKey) -> Option<usize> {
        match self.ps.get(k) {
            Some(v) => *v,
            None => self.base.as_ref()?.find(true, (&k.0, k.1, k.2)),
        }
    }

    pub fn ps_insert(&mut self, k: PsKey, v: usize) {
        self.ps.insert(k, Some(v));
    }

    pub fn ps_remove(&mut self, k: &PsKey) {
        if self
            .base
            .as_ref()
            .is_some_and(|b| b.find(true, (&k.0, k.1, k.2)).is_some())
        {
            self.ps.insert(k.clone(), None);
        } else {
            self.ps.remove(k);
        }
    }

    /// `tfm_tree` as the plain map.
    pub fn tfm_tree(&self) -> BTreeMap<Vec<u8>, usize> {
        let mut m: BTreeMap<Vec<u8>, usize> = match &self.base {
            Some(b) => b.tfm_elements().map(|(k, v)| (k.to_vec(), v)).collect(),
            None => BTreeMap::new(),
        };
        for (k, v) in self.tfm.iter() {
            match v {
                Some(v) => m.insert(k.clone(), *v),
                None => m.remove(k),
            };
        }
        m
    }

    /// `ps_tree` as the plain map.
    pub fn ps_tree(&self) -> BTreeMap<PsKey, usize> {
        let mut m: BTreeMap<PsKey, usize> = match &self.base {
            Some(b) => b
                .ps_elements()
                .map(|((k, s, e), v)| ((k.to_vec(), s, e), v))
                .collect(),
            None => BTreeMap::new(),
        };
        for (k, v) in self.ps.iter() {
            match v {
                Some(v) => m.insert(k.clone(), *v),
                None => m.remove(k),
            };
        }
        m
    }

    /// The entries as the plain vector.
    pub fn entries(&self) -> Vec<Option<FmEntry>> {
        let nb = self.base_len();
        let mut v: Vec<Option<FmEntry>> = (0..nb)
            .map(|i| match self.changed.get(&i) {
                Some(e) => e.clone(),
                None => self.base.as_ref().unwrap().decode(i),
            })
            .collect();
        v.extend(self.added.iter().cloned());
        v
    }

    /// The same table in content: the same copy, or, when not, equal
    /// plain forms.
    pub fn same_as(&self, o: &FmTable) -> bool {
        let base_same = match (&self.base, &o.base) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if base_same
            && (Shared::ptr_eq(&self.changed, &o.changed)
                || enc(&*self.changed) == enc(&*o.changed))
            && (Shared::ptr_eq(&self.added, &o.added) || enc(&*self.added) == enc(&*o.added))
            && (Shared::ptr_eq(&self.tfm, &o.tfm) || *self.tfm == *o.tfm)
            && (Shared::ptr_eq(&self.ps, &o.ps) || *self.ps == *o.ps)
        {
            return true;
        }
        enc(&self.entries()) == enc(&o.entries())
            && self.tfm_tree() == o.tfm_tree()
            && self.ps_tree() == o.ps_tree()
    }

    /// A table from its plain parts (a decoded persisted state), compact:
    /// a base of all of it.
    pub fn from_plain(
        fms: Vec<Option<FmEntry>>,
        tfm: BTreeMap<Vec<u8>, usize>,
        ps: BTreeMap<PsKey, usize>,
    ) -> Result<FmTable, String> {
        if fms.is_empty() && tfm.is_empty() && ps.is_empty() {
            return Ok(FmTable::default());
        }
        let mut w = vec![];
        encode(
            fms.iter().map(Option::as_ref),
            tfm.iter().map(|(k, &v)| (k.as_slice(), v)),
            ps.iter().map(|(k, &v)| (k, v)),
            &mut w,
        );
        let base = Base::owned(w).ok_or("persisted font map: an entry out of range")?;
        Ok(FmTable::from_base(Arc::new(base)))
    }

    /// The compact form of the whole table (its entries and trees).
    pub fn encode_compact(&self, w: &mut Vec<u8>) {
        let fms = self.entries();
        let tfm = self.tfm_tree();
        let ps = self.ps_tree();
        encode(
            fms.iter().map(Option::as_ref),
            tfm.iter().map(|(k, &v)| (k.as_slice(), v)),
            ps.iter().map(|(k, &v)| (k, v)),
            w,
        );
    }
}

fn enc<T: crate::persist::Codec>(x: &T) -> Vec<u8> {
    let mut w = vec![];
    x.enc(&mut w);
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, ps: Option<&str>, links: u16) -> FmEntry {
        let mut e = FmEntry::new();
        e.tfm_name = name.as_bytes().to_vec();
        e.ps_name = ps.map(|p| p.as_bytes().to_vec());
        e.ff_name = Some(format!("{name}.pfb").into_bytes());
        e.links = links;
        e.typ = 0x11;
        e.subfont = (name == "sub").then(|| vec![1, -1, 7]);
        e
    }

    type Plain = (
        Vec<Option<FmEntry>>,
        BTreeMap<Vec<u8>, usize>,
        BTreeMap<PsKey, usize>,
    );

    fn plain() -> Plain {
        let fms = vec![
            Some(entry("cmr10", Some("CMR10"), 3)),
            None,
            Some(entry("ptmr8r", None, 1)),
            Some(entry("sub", Some("Sub"), 1)),
        ];
        let tfm: BTreeMap<Vec<u8>, usize> = [
            (b"cmr10".to_vec(), 0),
            (b"ptmr8r".to_vec(), 2),
            (b"sub".to_vec(), 3),
        ]
        .into_iter()
        .collect();
        let ps: BTreeMap<PsKey, usize> = [
            ((b"CMR10".to_vec(), 0, 0), 0),
            ((b"CMR10".to_vec(), 167, 0), 0),
        ]
        .into_iter()
        .collect();
        (fms, tfm, ps)
    }

    #[test]
    fn a_base_reads_as_the_plain_parse() {
        let (fms, tfm, ps) = plain();
        let t = FmTable::from_plain(fms.clone(), tfm.clone(), ps.clone()).unwrap();
        assert_eq!(enc(&t.entries()), enc(&fms));
        assert_eq!(t.tfm_tree(), tfm);
        assert_eq!(t.ps_tree(), ps);
        assert_eq!(t.tfm_get(b"ptmr8r"), Some(2));
        assert_eq!(t.tfm_get(b"ptmr8"), None);
        assert_eq!(t.ps_get(&(b"CMR10".to_vec(), 167, 0)), Some(0));
        assert_eq!(t.ps_get(&(b"CMR10".to_vec(), 0, 1)), None);
        assert!(t[1].is_none());
        assert_eq!(t[3].as_ref().unwrap().subfont, Some(vec![1, -1, 7]));
    }

    #[test]
    fn changes_over_a_base_read_as_the_plain_changes() {
        let (mut fms, mut tfm, mut ps) = plain();
        let mut t = FmTable::from_plain(fms.clone(), tfm.clone(), ps.clone()).unwrap();
        let shared = t.clone();
        // replace cmr10, delete a ps element, add an entry
        t.tfm_remove(b"cmr10");
        tfm.remove(b"cmr10".as_slice());
        t.get_mut(0).as_mut().unwrap().links &= !1;
        fms[0].as_mut().unwrap().links &= !1;
        let k = (b"CMR10".to_vec(), 167, 0);
        t.ps_remove(&k);
        ps.remove(&k);
        t.tfm_insert(b"cmr10".to_vec(), 4);
        tfm.insert(b"cmr10".to_vec(), 4);
        t.push(Some(entry("cmr10", None, 1)));
        fms.push(Some(entry("cmr10", None, 1)));
        *t.get_mut(2) = None;
        fms[2] = None;
        assert_eq!(t.len(), 5);
        assert_eq!(enc(&t.entries()), enc(&fms));
        assert_eq!(t.tfm_tree(), tfm);
        assert_eq!(t.ps_tree(), ps);
        assert_eq!(t.tfm_get(b"cmr10"), Some(4));
        assert_eq!(t.ps_get(&k), None);
        // the checkpoint's copy is untouched
        assert_eq!(shared.tfm_get(b"cmr10"), Some(0));
        assert!(shared[2].is_some());
        assert!(!t.same_as(&shared));
        // the same content in another form is the same table
        let again = FmTable::from_plain(fms, tfm, ps).unwrap();
        assert!(t.same_as(&again));
    }

    #[test]
    fn a_damaged_compact_form_is_refused() {
        let (fms, tfm, ps) = plain();
        let mut w = vec![];
        encode(
            fms.iter().map(Option::as_ref),
            tfm.iter().map(|(k, &v)| (k.as_slice(), v)),
            ps.iter().map(|(k, &v)| (k, v)),
            &mut w,
        );
        assert!(Base::owned(w.clone()).is_some());
        for cut in 0..w.len() {
            assert!(Base::owned(w[..cut].to_vec()).is_none(), "cut at {cut}");
        }
        let mut longer = w.clone();
        longer.push(0);
        assert!(Base::owned(longer).is_none());
    }
}
