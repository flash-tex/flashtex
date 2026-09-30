//! L5 read-sets (DESIGN.md §5.5): which control sequences a run read, and
//! what a changed `.aux` changes.
//!
//! * **Recording.** From the `.aux` point on (`rs_on`), `changes/readset.ch`
//!   calls [`Globals::flashtex_cs_read`] the first time `get_next` reads a
//!   control sequence's meaning, and [`Globals::flashtex_id_read`] for
//!   every `id_lookup` (`\csname`, `\ifcsname`, a name scanned from a file),
//!   found or not. Each first read appends an [`Event`] (the name, keyed by
//!   its bytes, and the `eqtb` slot) to the [`ReadSet`] in the checkpoint
//!   layer; `rs_seen` (in the word space, so restores put it back) marks
//!   the slots already logged. Every checkpoint records the log's length
//!   (`ExtRecord::rs`), so "the checkpoints before the first read of any of
//!   these names" is a prefix of the chain.
//! * **What a changed `.aux` changes.** A pass whose `.aux` differs from the
//!   one the old run read re-runs the `.aux` read alone (from the `.aux`
//!   point to the first `big_switch` after the file closes, `Point::AuxDone`)
//!   and compares the state there with the old run's ([`aux_delta`]):
//!   every differing word must be a control sequence's meaning (or the
//!   memory, names and strings those meanings take); the names whose
//!   meaning differs, compared by content (token lists by their tokens,
//!   control sequences by name, so where the runs allocated does not
//!   matter), are the changed entries, with their new meanings.
//! * **Patching.** The pass restarts at the newest checkpoint before the
//!   first read of a changed entry, puts the new meanings in
//!   ([`apply_patch`]: `eq_destroy` the old, build the new token list with
//!   `get_avail`, insert names with `id_lookup`), and runs on. The state is
//!   then the from-scratch run's up to where its dynamic memory, hash slots
//!   and string numbers went, which nothing a document prints depends on.

use crate::arena::ChunkDiff;
use crate::generated::types::memory_word;
use crate::generated::Globals;
use std::collections::{HashMap, HashSet};

/// `hash_base`, `undefined_control_sequence`, `frozen_control_sequence`,
/// `single_base`, `null_cs` (pdftex.web §222 with this build's sizes).
const HASH_BASE: i32 = 514;
const SINGLE_BASE: i32 = 257;
const NULL_CS: i32 = 513;
const FROZEN_CONTROL_SEQUENCE: i32 = 615_514;
pub const UNDEFINED_CONTROL_SEQUENCE: i32 = 626_627;
const HASH_PRIME: i64 = 522_749;
/// `cs_token_flag` (§289).
const CS_TOKEN_FLAG: i32 = 4095;
/// `eq_type` codes (§209, §210 with this build's `max_command`).
const RELAX: i32 = 0;
const UNDEFINED_CS: i32 = 104;
const CALL: i32 = 114;
const LONG_OUTER_CALL: i32 = 117;
const LEVEL_ZERO: i32 = 0;
const LEVEL_ONE: i32 = 1;
/// A token list longer than this is not a `.aux` entry (and a cycle).
const MAX_TOKENS: usize = 1 << 20;

/// A first read: the name's key and its slot (0 for a name looked up and
/// not found).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub name: u64,
    pub p: i32,
}

/// The first reads of a run, in order (see the module comment).
#[derive(Clone, Debug, Default)]
pub struct ReadSet {
    pub events: Vec<Event>,
    counts: HashMap<u64, u32>,
}

impl ReadSet {
    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    fn push(&mut self, name: u64, p: i32) {
        self.events.push(Event { name, p });
        *self.counts.entry(name).or_default() += 1;
    }

    fn logged(&self, name: u64) -> bool {
        self.counts.contains_key(&name)
    }

    /// Back to the first `n` events (a restore to a checkpoint taken then).
    pub fn truncate(&mut self, n: usize) {
        if n >= self.events.len() {
            return;
        }
        for e in &self.events[n..] {
            if let Some(c) = self.counts.get_mut(&e.name) {
                *c -= 1;
                if *c == 0 {
                    self.counts.remove(&e.name);
                }
            }
        }
        self.events.truncate(n);
    }

    /// A read-set holding `events`.
    pub fn from_events(events: Vec<Event>) -> ReadSet {
        let mut counts: HashMap<u64, u32> = HashMap::new();
        for e in &events {
            *counts.entry(e.name).or_default() += 1;
        }
        ReadSet { events, counts }
    }

    /// The first event at or after `from` that reads one of `names`.
    pub fn first_read(&self, from: usize, names: &HashSet<u64>) -> Option<usize> {
        self.events
            .iter()
            .enumerate()
            .skip(from)
            .find(|(_, e)| names.contains(&e.name))
            .map(|(i, _)| i)
    }
}

/// The key of a name: FNV-1a over a kind byte and the name's bytes.
fn key(kind: u8, bytes: impl Iterator<Item = u8>) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut step = |b: u8| {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    };
    step(kind);
    for b in bytes {
        step(b);
    }
    h
}

/// A control sequence's name, as the two runs can compare it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Name {
    /// A multi-letter name (a hash slot).
    Multi(Vec<u8>),
    /// `\x` for one character, an active character, `\csname\endcsname`.
    Single(u8),
    Active(u8),
    Null,
    /// A slot at a fixed place (frozen control sequences, font
    /// identifiers, `undefined_control_sequence`).
    Fixed(i32),
}

impl Name {
    pub fn key(&self) -> u64 {
        match self {
            Name::Multi(b) => key(b'M', b.iter().copied()),
            Name::Single(c) => key(b'S', std::iter::once(*c)),
            Name::Active(c) => key(b'A', std::iter::once(*c)),
            Name::Null => key(b'N', std::iter::empty()),
            Name::Fixed(p) => key(b'F', p.to_le_bytes().into_iter()),
        }
    }
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Name::Multi(b) => write!(f, "\\{}", String::from_utf8_lossy(b)),
            Name::Single(c) => write!(f, "\\{}", *c as char),
            Name::Active(c) => write!(f, "active {}", *c as char),
            Name::Null => write!(f, "\\csname\\endcsname"),
            Name::Fixed(p) => write!(f, "eqtb[{p}]"),
        }
    }
}

/// Read access to one engine state: the live one, or an old run's
/// checkpoint through a `ChunkDiff`.
pub struct View<'a> {
    g: &'a Globals,
    old: Option<&'a ChunkDiff>,
    off: Offsets,
}

#[derive(Clone, Copy)]
struct Offsets {
    eqtb: usize,
    hash: usize,
    mem: usize,
    str_pool: usize,
    str_start: usize,
}

fn offsets(g: &Globals) -> Result<Offsets, String> {
    let f = |n: &str| {
        g.arena
            .regions
            .iter()
            .find(|r| r.name == n)
            .map(|r| r.off)
            .ok_or_else(|| format!("no region {n}"))
    };
    Ok(Offsets {
        eqtb: f("eqtb")?,
        hash: f("hash")?,
        mem: f("mem")?,
        str_pool: f("str_pool")?,
        str_start: f("str_start")?,
    })
}

impl<'a> View<'a> {
    pub fn live(g: &'a Globals) -> Result<View<'a>, String> {
        Ok(View {
            g,
            old: None,
            off: offsets(g)?,
        })
    }

    pub fn old(g: &'a Globals, d: &'a ChunkDiff) -> Result<View<'a>, String> {
        Ok(View {
            g,
            old: Some(d),
            off: offsets(g)?,
        })
    }

    fn word(&self, off: usize) -> u64 {
        match self.old {
            Some(d) => d.old_word(&self.g.arena, off & !7),
            None => {
                let b = self.g.arena.read(off & !7, 8);
                u64::from_le_bytes(b.try_into().unwrap())
            }
        }
    }

    fn int4(&self, off: usize) -> i32 {
        let w = self.word(off);
        ((w >> ((off & 7) * 8)) & 0xFFFF_FFFF) as u32 as i32
    }

    /// `eqtb[p]` (pdftex.web's index).
    pub fn eqtb(&self, p: i32) -> u64 {
        self.word(self.off.eqtb + (p as usize - 1) * 8)
    }

    /// `hash[p]`: (`next`, `text`).
    pub fn hash(&self, p: i32) -> (i32, i32) {
        let w = self.word(self.off.hash + (p - HASH_BASE) as usize * 8);
        ((w >> 32) as u32 as i32, w as u32 as i32)
    }

    /// `mem[p]`: (`info`, `link`).
    pub fn mem(&self, p: i32) -> (i32, i32) {
        let w = self.word(self.off.mem + p as usize * 8);
        ((w >> 32) as u32 as i32, w as u32 as i32)
    }

    /// A 4-byte scalar global by name.
    pub fn scalar_i32(&self, name: &str) -> Option<i32> {
        let s = layout().iter().find(|s| s.name == name)?;
        Some(self.int4(s.off))
    }

    pub fn str_ptr(&self) -> i32 {
        self.scalar_i32("str_ptr").unwrap_or(0)
    }

    pub fn string_bytes(&self, s: i32) -> Vec<u8> {
        self.string(s)
    }

    fn string(&self, s: i32) -> Vec<u8> {
        if s <= 0 {
            return vec![];
        }
        let a = self.int4(self.off.str_start + s as usize * 4);
        let b = self.int4(self.off.str_start + (s as usize + 1) * 4);
        (a..b.max(a))
            .map(|k| self.int4(self.off.str_pool + k as usize * 4) as u8)
            .collect()
    }

    /// The name of the control sequence at `p` (`None`: an empty hash slot).
    pub fn name(&self, p: i32) -> Option<Name> {
        if p < SINGLE_BASE {
            Some(Name::Active((p - 1) as u8))
        } else if p < NULL_CS {
            Some(Name::Single((p - SINGLE_BASE) as u8))
        } else if p == NULL_CS {
            Some(Name::Null)
        } else if p < FROZEN_CONTROL_SEQUENCE {
            let (_, t) = self.hash(p);
            (t > 0).then(|| Name::Multi(self.string(t)))
        } else {
            Some(Name::Fixed(p))
        }
    }

    /// Where `name` is (pdftex.web §259's search, without inserting).
    pub fn lookup(&self, name: &Name) -> Option<i32> {
        match name {
            Name::Active(c) => Some(*c as i32 + 1),
            Name::Single(c) => Some(*c as i32 + SINGLE_BASE),
            Name::Null => Some(NULL_CS),
            Name::Fixed(p) => Some(*p),
            Name::Multi(b) => {
                if b.len() < 2 {
                    return None;
                }
                let mut h: i64 = b[0] as i64;
                for &c in &b[1..] {
                    h = h + h + c as i64;
                    while h >= HASH_PRIME {
                        h -= HASH_PRIME;
                    }
                }
                let mut p = h as i32 + HASH_BASE;
                for _ in 0..1_000_000 {
                    let (next, text) = self.hash(p);
                    if text > 0 && self.string(text) == *b {
                        return Some(p);
                    }
                    if next == 0 {
                        return None;
                    }
                    p = next;
                }
                None
            }
        }
    }

    /// The meaning of the control sequence at `p`.
    pub fn meaning(&self, p: i32) -> Result<Meaning, String> {
        let w = self.eqtb(p);
        let ty = (w >> 48) as i32;
        let level = ((w >> 32) & 0xFFFF) as i32;
        let equiv = w as u32 as i32;
        if ty == UNDEFINED_CS && level == LEVEL_ZERO && equiv == 0 {
            return Ok(Meaning::Undefined);
        }
        if (CALL..=LONG_OUTER_CALL).contains(&ty) {
            let mut toks = vec![];
            let (_, mut q) = self.mem(equiv); // the reference count
            while q != 0 {
                if toks.len() > MAX_TOKENS {
                    return Err(format!("{}: token list too long", self.describe(p)));
                }
                let (t, next) = self.mem(q);
                if t >= CS_TOKEN_FLAG {
                    let c = t - CS_TOKEN_FLAG;
                    let n = self.name(c).ok_or_else(|| {
                        format!("a token of {} names an empty slot", self.describe(p))
                    })?;
                    toks.push(Tok::Cs(n));
                } else {
                    toks.push(Tok::Char(t));
                }
                q = next;
            }
            return Ok(Meaning::Macro { ty, level, toks });
        }
        Ok(Meaning::Value { ty, level, equiv })
    }

    fn describe(&self, p: i32) -> String {
        self.name(p)
            .map_or_else(|| format!("slot {p}"), |n| n.to_string())
    }
}

/// The scalar globals' layout (it depends on the generated code only).
pub fn layout() -> &'static [crate::statediff::ScalarSlot] {
    static L: std::sync::OnceLock<Vec<crate::statediff::ScalarSlot>> = std::sync::OnceLock::new();
    L.get_or_init(|| crate::statediff::scalar_layout(&mut Globals::new()))
}

/// A token of a macro's body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tok {
    Char(i32),
    Cs(Name),
}

/// A control sequence's meaning, comparable across runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Meaning {
    Undefined,
    /// A macro (`call` .. `long_outer_call`) and its body.
    Macro {
        ty: i32,
        level: i32,
        toks: Vec<Tok>,
    },
    /// Anything else: the `eqtb` word's fields as they are.
    Value {
        ty: i32,
        level: i32,
        equiv: i32,
    },
}

/// New meanings for names (a changed `.aux`'s effect), in the order to
/// apply them.
#[derive(Clone, Debug, Default)]
pub struct Patch {
    pub defs: Vec<(Name, Meaning)>,
}

impl Patch {
    pub fn keys(&self) -> HashSet<u64> {
        self.defs.iter().map(|(n, _)| n.key()).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }

    /// This patch, then `later` (a later pass's) over it.
    pub fn then(&self, later: &Patch) -> Patch {
        let mut defs: Vec<(Name, Meaning)> = self
            .defs
            .iter()
            .filter(|(n, _)| !later.defs.iter().any(|(m, _)| m == n))
            .cloned()
            .collect();
        defs.extend(later.defs.iter().cloned());
        Patch { defs }
    }

    fn get(&self, n: &Name) -> Option<&Meaning> {
        self.defs.iter().find(|(m, _)| m == n).map(|(_, x)| x)
    }
}

/// What a re-read `.aux` changed (the module comment): the live state is
/// the new run at `Point::AuxDone`, `d` its diff with the old run's state
/// there, `before` the meanings earlier passes already patched into the
/// old run's later checkpoints. Returns the names whose meaning differs
/// from the old run's (with `before`), with their new meanings, and the
/// patch that puts the old run's own meanings back into the live state:
/// the caller applies that and compares the states (`incr::same_words`),
/// which proves that nothing else differs. `Err`: a difference this cannot
/// describe (the caller re-runs from the `.aux` read instead).
pub fn aux_delta(
    g: &mut Globals,
    d: &ChunkDiff,
    before: &Patch,
    dead: &dyn Fn(&Globals, &crate::statediff::WordDiff) -> bool,
) -> Result<(Patch, Patch), String> {
    let words = crate::statediff::words(g, d);
    let g: &Globals = g;
    let mut slots: Vec<i32> = vec![];
    let mut mem_words: Vec<usize> = vec![];
    for w in &words {
        if w.region == "rs_seen" || dead(g, w) {
            continue;
        }
        match (w.region, w.scalar) {
            ("eqtb", None) => {
                let p = w.index as i32 + 1;
                if p < UNDEFINED_CONTROL_SEQUENCE {
                    slots.push(p);
                }
            }
            ("hash", None) => slots.push(w.index as i32 + HASH_BASE),
            ("mem", None) => mem_words.push(w.index),
            _ => {}
        }
    }
    let old = View::old(g, d)?;
    let new = View::live(g)?;
    // A macro whose `eqtb` word is the same in both states may still hold
    // another body there (both runs allocated its list at the same place):
    // the control sequences, in the chunks of `eqtb` either run wrote,
    // whose list holds a differing cell of `mem`; and a list another
    // control sequence shares (`\let`) has a reference count the read may
    // have changed: the owners, anywhere in `eqtb`, of a differing cell
    // that heads a list (where neither run wrote, the old state is the
    // live one).
    if !mem_words.is_empty() {
        let differing: HashSet<i32> = mem_words.iter().map(|&p| p as i32).collect();
        let eqtb_r = g
            .arena
            .regions
            .iter()
            .find(|r| r.name == "eqtb")
            .ok_or("no eqtb")?;
        let (lo, hi) = (eqtb_r.off, eqtb_r.off + eqtb_r.bytes);
        let cb = crate::arena::CHUNK_BYTES;
        let holds = |v: &View, p: i32| -> bool {
            let w = v.eqtb(p);
            let ty = (w >> 48) as i32;
            if !(CALL..=LONG_OUTER_CALL).contains(&ty) {
                return false;
            }
            let mut q = w as u32 as i32;
            let mut n = 0;
            while q != 0 && n <= MAX_TOKENS {
                if differing.contains(&q) {
                    return true;
                }
                q = v.mem(q).1;
                n += 1;
            }
            false
        };
        for c in d.written() {
            let (a, b) = (c as usize * cb, (c as usize + 1) * cb);
            if b <= lo || a >= hi {
                continue;
            }
            let first = (a.max(lo) - lo) / 8 + 1;
            let last = (b.min(hi) - lo) / 8;
            for p in first as i32..=last as i32 {
                if p >= UNDEFINED_CONTROL_SEQUENCE {
                    break;
                }
                if holds(&old, p) || holds(&new, p) {
                    slots.push(p);
                }
            }
        }
        for p in 1..UNDEFINED_CONTROL_SEQUENCE {
            let w = g.eqtb[(p - 1) as usize].to_bits();
            let ty = (w >> 48) as i32;
            if (CALL..=LONG_OUTER_CALL).contains(&ty) && differing.contains(&(w as u32 as i32)) {
                slots.push(p);
            }
        }
    }
    // Every name either state has at a differing slot, and every name an
    // earlier pass patched.
    let mut names: Vec<Name> = vec![];
    for &p in &slots {
        for n in [old.name(p), new.name(p)].into_iter().flatten() {
            if !names.contains(&n) {
                names.push(n);
            }
        }
    }
    for (n, _) in &before.defs {
        if !names.contains(n) {
            names.push(n.clone());
        }
    }
    let mut patch = Patch::default();
    let mut back = Patch::default();
    for n in &names {
        let raw = match old.lookup(n) {
            Some(p) => old.meaning(p)?,
            None => Meaning::Undefined,
        };
        let mo = before.get(n).cloned().unwrap_or_else(|| raw.clone());
        let mn = match new.lookup(n) {
            Some(p) => new.meaning(p)?,
            None => Meaning::Undefined,
        };
        if mo != mn {
            patch.defs.push((n.clone(), mn.clone()));
        }
        if raw != mn {
            back.defs.push((n.clone(), raw));
        }
    }
    Ok((patch, back))
}

/// Renumber the strings the live state made since string `from` so that
/// they are in the order `olds` (another run's strings from `from` on,
/// the same strings), and the hash's names with them: an `.aux` whose
/// lines came in another order (a `\bibcite` written before a label's
/// page shipped) made the same names in another order. Only the hash is
/// renumbered; any other reference to such a string stays as it is, and
/// the comparison that follows sees it. Names the live read made and the
/// old one did not (new labels, undefined again by then) leave the hash and
/// the pool, with `cs_count` and `hash_used` the old run's (`old_counts`).
/// `Err` if the old read made a name the live one did not, or a new name
/// cannot leave the hash that way.
pub fn permute_strings(
    g: &mut Globals,
    from: i32,
    olds: &[Vec<u8>],
    old_counts: (i32, i32),
) -> Result<(), String> {
    let v = View::live(g)?;
    let sn = g.str_ptr;
    let news: Vec<Vec<u8>> = (from..sn).map(|s| v.string(s)).collect();
    if news == olds {
        return Ok(());
    }
    // the names the new read made and the old one did not (new labels):
    // undefined now (the caller put the old meanings back), they leave the
    // hash, as they are not in the old run's
    let mut left: HashMap<&[u8], usize> = HashMap::new();
    for s in olds {
        *left.entry(s.as_slice()).or_default() += 1;
    }
    let mut extra: Vec<Vec<u8>> = vec![];
    for s in &news {
        match left.get_mut(s.as_slice()) {
            Some(n) if *n > 0 => *n -= 1,
            _ => extra.push(s.clone()),
        }
    }
    if left.values().any(|&n| n > 0) {
        return Err(format!(
            "the old .aux read made names the new one did not ({} strings, the old run {})",
            news.len(),
            olds.len()
        ));
    }
    for s in &extra {
        let name = Name::Multi(s.clone());
        let Some(p) = View::live(g)?.lookup(&name) else {
            return Err(format!(
                "the new string {} is not a name",
                String::from_utf8_lossy(s)
            ));
        };
        if View::live(g)?.meaning(p)? != Meaning::Undefined {
            return Err(format!("{name} is defined"));
        }
        // unlink it: from the chain of its first slot, which it must not head
        let mut h: i64 = s[0] as i64;
        for &c in &s[1..] {
            h = h + h + c as i64;
            while h >= HASH_PRIME {
                h -= HASH_PRIME;
            }
        }
        let first = h as i32 + HASH_BASE;
        let idx = |q: i32| (q - HASH_BASE) as usize;
        if p == first {
            if g.hash[idx(p)].lh() != 0 {
                return Err(format!("{name} heads a chain of the hash"));
            }
        } else {
            let mut q = first;
            loop {
                let next = g.hash[idx(q)].lh();
                if next == p {
                    let after = g.hash[idx(p)].lh();
                    g.hash[idx(q)].set_lh(after);
                    break;
                }
                if next == 0 {
                    return Err(format!("{name} is not in its chain"));
                }
                q = next;
            }
            g.hash[idx(p)].set_lh(0);
        }
        g.hash[idx(p)].set_rh(0);
    }
    // the strings in the old run's order, the new ones after them (then
    // dropped); the hash's names renumbered
    let mut at: HashMap<&[u8], Vec<i32>> = HashMap::new();
    let order: Vec<&Vec<u8>> = olds.iter().chain(extra.iter()).collect();
    for (j, s) in order.iter().enumerate().rev() {
        at.entry(s.as_slice()).or_default().push(from + j as i32);
    }
    let map: Vec<i32> = news
        .iter()
        .map(|s| at.get_mut(s.as_slice()).and_then(|v| v.pop()).unwrap_or(0))
        .collect();
    let mut pos = g.str_start[from as usize];
    let mut old_end = pos;
    for (j, s) in order.iter().enumerate() {
        g.str_start[from as usize + j] = pos;
        for &c in s.iter() {
            g.str_pool[pos as usize] = c as i32;
            pos += 1;
        }
        if j + 1 == olds.len() {
            old_end = pos;
        }
    }
    g.str_start[sn as usize] = pos;
    for i in 0..g.hash.len() {
        let t = g.hash[i].rh();
        if t >= from && t < sn {
            let n = map[(t - from) as usize];
            g.hash[i].set_rh(n);
        }
    }
    if !extra.is_empty() {
        let n = from + olds.len() as i32;
        g.str_start[n as usize] = if olds.is_empty() {
            g.str_start[from as usize]
        } else {
            old_end
        };
        g.pool_ptr = g.str_start[n as usize];
        g.str_ptr = n;
        g.cs_count = old_counts.0;
        g.hash_used = old_counts.1;
    }
    Ok(())
}

/// Put `patch`'s meanings into the live state (the module comment).
pub fn apply_patch(g: &mut Globals, patch: &Patch) -> Result<(), String> {
    let rs = g.rs_on;
    g.rs_on = false;
    let r = apply(g, patch);
    g.rs_on = rs;
    r
}

fn apply(g: &mut Globals, patch: &Patch) -> Result<(), String> {
    for (name, m) in &patch.defs {
        let found = View::live(g)?.lookup(name);
        let p = match (found, m) {
            (Some(p), _) => p,
            (None, Meaning::Undefined) => continue,
            (None, _) => insert(g, name)?,
        };
        // a meaning saved by a group still open: the group's end would
        // restore it (or keep the global one), which a patch cannot know
        for k in 0..g.save_ptr.max(0) as usize {
            let e = g.save_stack[k].hh();
            if (e.b0() == 0 || e.b0() == 1) && e.rh() == p {
                return Err(format!("{name} is saved by an open group"));
            }
        }
        let w = match m {
            Meaning::Undefined => word(UNDEFINED_CS, LEVEL_ZERO, 0),
            Meaning::Value { ty, level, equiv } => {
                if *ty != RELAX && *ty != UNDEFINED_CS && *ty > 70 {
                    // (a register, a font, a box: not what an .aux defines)
                    return Err(format!("{name}: meaning of type {ty} not patched"));
                }
                word(*ty, *level, *equiv)
            }
            Meaning::Macro { ty, level, toks } => {
                if *level != LEVEL_ONE {
                    return Err(format!("{name}: a local definition (level {level})"));
                }
                let r = g.get_avail();
                g.mem[r as usize].set_hh_lh(0);
                let mut tail = r;
                for t in toks {
                    let v = match t {
                        Tok::Char(c) => *c,
                        Tok::Cs(n) => {
                            let q = match View::live(g)?.lookup(n) {
                                Some(q) => q,
                                None => insert(g, n)?,
                            };
                            CS_TOKEN_FLAG + q
                        }
                    };
                    let q = g.get_avail();
                    g.mem[q as usize].set_hh_lh(v);
                    g.mem[tail as usize].set_hh_rh(q);
                    tail = q;
                }
                word(*ty, *level, r)
            }
        };
        let old = g.eqtb[(p - 1) as usize];
        g.eq_destroy(old);
        g.eqtb[(p - 1) as usize] = w;
        // a guarded intrinsic that read this meaning must see it change
        // (src/intrinsics.rs; eq_define reports its own writes)
        if g.intr_watch[p as usize] != 0 {
            g.flashtex_intr_touch(p);
        }
    }
    Ok(())
}

fn word(ty: i32, level: i32, equiv: i32) -> memory_word {
    let mut w = memory_word(0);
    w.set_hh_b0(ty);
    w.set_hh_b1(level);
    w.set_hh_rh(equiv);
    w
}

/// Enter a multi-letter name into the hash (`id_lookup`, from the free end
/// of `buffer`).
fn insert(g: &mut Globals, name: &Name) -> Result<i32, String> {
    let Name::Multi(b) = name else {
        return Err(format!("{name} has no slot"));
    };
    let j = g.first.max(0);
    if j as usize + b.len() + 1 >= crate::generated::consts::buf_size as usize {
        return Err("no room in the buffer".into());
    }
    for (k, &c) in b.iter().enumerate() {
        g.buffer[j as usize + k] = c as i32;
    }
    let nn = g.no_new_control_sequence;
    g.no_new_control_sequence = false;
    let p = g.id_lookup(j, b.len() as i32);
    g.no_new_control_sequence = nn;
    Ok(p)
}

/// The key of the name of `eqtb` slot `p` in the live state.
fn name_key(g: &Globals, p: i32) -> u64 {
    match View::live(g).ok().and_then(|v| v.name(p)) {
        Some(n) => n.key(),
        None => Name::Fixed(p).key(),
    }
}

/// Set `rs_seen` from the read-set: the slots of its events whose name is
/// still theirs (after a jump the events of the new run's part name slots
/// of its own state).
pub fn rebuild_seen(g: &mut Globals) {
    let n = g.rs_seen.len();
    let zeros = vec![0u8; n];
    let off = g
        .arena
        .regions
        .iter()
        .find(|r| r.name == "rs_seen")
        .map(|r| r.off);
    let Some(off) = off else { return };
    g.arena.write_through(off, &zeros);
    let events = g.layer().rs.events.clone();
    for e in events {
        if e.p > 0 && (e.p as usize) < n && name_key(g, e.p) == e.name {
            g.rs_seen[e.p as usize] = true;
        }
    }
}

impl Globals {
    /// `get_next` read the meaning of `p` for the first time since the
    /// read-set began (`changes/readset.ch`).
    pub fn flashtex_cs_read(&mut self, p: i32) {
        let k = name_key(self, p);
        self.rs_seen[p as usize] = true;
        self.layer().rs.push(k, p);
    }

    /// `id_lookup` looked `buffer[j..j+l)` up and returns `p`
    /// (`undefined_control_sequence`: not found).
    pub fn flashtex_id_read(&mut self, j: i32, l: i32, p: i32) {
        let found = p != UNDEFINED_CONTROL_SEQUENCE;
        if found && self.rs_seen[p as usize] {
            return;
        }
        let k = key(b'M', (j..j + l).map(|i| self.buffer[i as usize] as u8));
        if found {
            self.rs_seen[p as usize] = true;
            self.layer().rs.push(k, p);
        } else {
            let l = self.layer();
            if !l.rs.logged(k) {
                l.rs.push(k, 0);
            }
        }
    }
}
