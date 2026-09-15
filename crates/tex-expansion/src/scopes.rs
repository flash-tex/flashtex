//! Unified grouping/save-stack for everything TeX assignments can touch
//! locally: control-sequence meanings (`\def`/`\let`/...), category codes
//! (`\catcode`), `\uccode`/`\lccode`, integer parameters (`\escapechar`),
//! and registers (`\count`/`\dimen`/`\skip`/`\toks`).
//! TeXbook ch. 24's "save stack": `{`/`}` and `\begingroup`/`\endgroup`
//! open/close a scope; assignments are local by default and are undone
//! when their scope closes; `\global` makes them permanent (no save
//! entry is pushed) and `\aftergroup` queues a token for reinsertion right
//! after the scope closes.
//!
//! Everything here is `Clone` in near-constant time (copy-on-write chunked
//! tables, see `CowMap`) so the incremental expander can snapshot the whole
//! assignment state at every checkpoint and compare two states in time
//! proportional to what changed between them.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::rc::Rc;

use crate::catcode::{CatCode, CatCodeTable};
use crate::macro_def::MacroDef;
use crate::registers::Glue;
use crate::span::Span;
use crate::token::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum Meaning {
    Macro(Rc<MacroDef>),
    Let(Box<Meaning>),
    CharLike(Token),
    Primitive(Primitive),
    RegisterAlias(RegisterKind, u16),
    /// `\chardef\x=<n>`: behaves as the character `<n>` when typeset and
    /// as the integer `<n>` in a `<number>` context (TeXbook p. 277).
    CharDef(i64),
    /// `\mathchardef\x=<n>`: a math character code; an integer in
    /// `<number>` contexts.
    MathCharDef(i64),
    Undefined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterKind {
    Count,
    Dimen,
    Skip,
    Toks,
}

/// Integer parameters this crate actually models (the rest of TeX's
/// `\tolerance`-style typesetting parameters are passed through to the
/// typesetter untouched).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntParam {
    Escapechar,
    Endlinechar,
    Newlinechar,
    /// e-TeX `\eTeXversion` (read-only in TeX; 2).
    ETeXVersion,
    /// Internal: the host's font selector (`FontSwitch`), scoped like
    /// TeX's current font. Not reachable from TeX source.
    Font,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Primitive {
    /// A host-typeset command (`Engine::declare_host_command`): defined, but
    /// emitted unchanged.
    Host,
    /// A host command that performs an assignment
    /// (`Engine::declare_host_assignment`): like `Host`, but `\global`
    /// is passed through ahead of it instead of being an error.
    HostAssignment,
    Relax,
    Par,
    Def,
    Edef,
    Gdef,
    Xdef,
    Let,
    Futurelet,
    Global,
    Long,
    Outer,
    Protected,
    Expandafter,
    Noexpand,
    Csname,
    Endcsname,
    String,
    Number,
    Romannumeral,
    MeaningOf,
    The,
    Unexpanded,
    Detokenize,
    /// pdfTeX/e-TeX 2019 `\expanded`.
    Expanded,
    // -- file/terminal I/O. `\input` reads through the host's file reader
    // when one is set (and otherwise passes through); the rest are
    // side-effect-free stand-ins so format code can be expanded.
    InputFile,
    Immediate,
    Write,
    Openout,
    Closeout,
    Openin,
    Closein,
    Read,
    Message,
    Errmessage,
    Jobname,
    /// e-TeX `\eTeXrevision` (expands to `.6`).
    ETeXRevision,
    /// pdfTeX `\pdfstrcmp` (XeTeX `\strcmp`).
    Pdfstrcmp,
    Scantokens,
    Afterassignment,
    Uppercase,
    Lowercase,
    Uccode,
    Lccode,
    Chardef,
    Mathchardef,
    IntPar(IntParam),
    Begingroup,
    Endgroup,
    Aftergroup,
    Catcode,
    Ignorespaces,
    Endinput,
    If,
    Ifcat,
    Ifx,
    Ifnum,
    Ifdim,
    Ifodd,
    Ifvmode,
    Ifhmode,
    Ifmmode,
    Ifinner,
    Ifcase,
    Iftrue,
    Iffalse,
    Ifdefined,
    Ifcsname,
    Ifhbox,
    Ifvbox,
    Ifvoid,
    Ifeof,
    Ifincsname,
    Or,
    Else,
    Fi,
    Newif,
    Unless,
    Count,
    Dimen,
    Skip,
    Toks,
    Countdef,
    Dimendef,
    Skipdef,
    Toksdef,
    Newcount,
    Newdimen,
    Newskip,
    Newtoks,
    Advance,
    Multiply,
    Divide,
    Numexpr,
    Dimexpr,
    // -- LaTeX layer (built on the primitives above) --
    NewCommand,
    RenewCommand,
    ProvideCommand,
    DeclareRobustCommand,
    NewEnvironment,
    RenewEnvironment,
    Begin,
    End,
    NewCounter,
    SetCounter,
    AddToCounter,
    StepCounter,
    RefStepCounter,
    AddToReset,
    RemoveFromReset,
    CounterWithin,
    CounterWithout,
    Label,
    Value,
    Arabic,
    RomanLower,
    RomanUpper,
    AlphLower,
    AlphUpper,
    Fnsymbol,
    NewLength,
    SetToWidth,
    SetToHeight,
    SetToDepth,
    DefineKey,
    SetKeys,
    /// `\verb` (reads raw characters from the source).
    Verb,
    /// Internal: stop reading all input (`\end{document}`).
    StopInput,
}

#[derive(Debug, Clone, PartialEq)]
enum SaveItem {
    CsMeaning(String, Meaning),
    Catcode(char, CatCode),
    Uccode(char, char),
    Lccode(char, char),
    IntParam(IntParam, i64),
    Count(u16, i64),
    Dimen(u16, i64),
    Skip(u16, Glue),
    Toks(u16, Vec<Token>),
}

#[derive(Debug, Clone, PartialEq)]
struct Frame {
    saves: Vec<SaveItem>,
    after_group: Vec<Token>,
    /// Upper bound on the `end` of every document span stored in this frame
    /// (see [`SpanBound`]).
    max_end: u32,
}

impl Frame {
    fn new() -> Self {
        Frame { saves: Vec::new(), after_group: Vec::new(), max_end: 0 }
    }
}

// ---- persistent storage -------------------------------------------------
//
// Incremental checkpoints snapshot the whole assignment state and compare an
// old run's state with a new run's (modulo a span shift). Every table is a
// two-level copy-on-write structure so both are proportional to what changed
// rather than to the state's size: a snapshot bumps a few reference counts,
// the first write to a chunk after a snapshot copies just that chunk, and a
// comparison skips every chunk that is the same allocation on both sides and
// holds no span an edit could move.

/// Hasher for the in-crate tables (FxHash): keys are control-sequence names
/// and small integers from the document, so SipHash's DoS resistance buys
/// nothing and costs a measurable share of every lookup.
#[derive(Default, Clone, Copy)]
pub(crate) struct FxHasher {
    hash: u64,
}

const FX_SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

impl FxHasher {
    #[inline]
    fn add(&mut self, word: u64) {
        self.hash = (self.hash.rotate_left(5) ^ word).wrapping_mul(FX_SEED);
    }
}

impl Hasher for FxHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(8);
        for c in &mut chunks {
            self.add(u64::from_le_bytes(c.try_into().unwrap()));
        }
        let rest = chunks.remainder();
        if !rest.is_empty() {
            let mut buf = [0u8; 8];
            buf[..rest.len()].copy_from_slice(rest);
            self.add(u64::from_le_bytes(buf));
        }
    }
    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.add(i as u64);
    }
    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.add(i as u64);
    }
    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.add(i as u64);
    }
    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.add(i);
    }
    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.add(i as u64);
    }
    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
}

pub(crate) type FxMap<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;

/// Largest `end` offset of a document (source id 0) span stored in a value.
/// Every span shift the incremental expander applies leaves a span with
/// `end <= edit_start` unchanged, so a value whose bound is at most the
/// smallest edit start involved maps to itself.
pub(crate) trait SpanBound {
    fn max_end(&self) -> u32;
}

fn span_end(s: Span) -> u32 {
    if s.is_synthetic() || s.source_id != 0 {
        0
    } else {
        s.end
    }
}

fn tokens_end(ts: &[Token]) -> u32 {
    ts.iter().map(|t| span_end(t.span)).max().unwrap_or(0)
}

impl SpanBound for Meaning {
    fn max_end(&self) -> u32 {
        use crate::macro_def::{BodyPart, ParamPart};
        match self {
            Meaning::Macro(d) => {
                let p = d.params.iter().map(|p| if let ParamPart::Literal(t) = p { span_end(t.span) } else { 0 }).max().unwrap_or(0);
                let b = d.body.iter().map(|p| if let BodyPart::Literal(t) = p { span_end(t.span) } else { 0 }).max().unwrap_or(0);
                p.max(b)
            }
            Meaning::Let(inner) => inner.max_end(),
            Meaning::CharLike(t) => span_end(t.span),
            _ => 0,
        }
    }
}

impl SpanBound for Vec<Token> {
    fn max_end(&self) -> u32 {
        tokens_end(self)
    }
}

impl SpanBound for i64 {
    fn max_end(&self) -> u32 {
        0
    }
}

impl SpanBound for char {
    fn max_end(&self) -> u32 {
        0
    }
}

impl SpanBound for Glue {
    fn max_end(&self) -> u32 {
        0
    }
}

impl SpanBound for SaveItem {
    fn max_end(&self) -> u32 {
        match self {
            SaveItem::CsMeaning(_, m) => m.max_end(),
            SaveItem::Toks(_, v) => tokens_end(v),
            _ => 0,
        }
    }
}

/// Which chunk a key lives in.
pub(crate) trait ChunkKey: Eq + Hash + Clone {
    fn chunk_hash(&self) -> u64;
}

impl ChunkKey for String {
    fn chunk_hash(&self) -> u64 {
        let mut h = FxHasher::default();
        h.write(self.as_bytes());
        h.finish()
    }
}

impl ChunkKey for char {
    fn chunk_hash(&self) -> u64 {
        *self as u64
    }
}

impl ChunkKey for u16 {
    fn chunk_hash(&self) -> u64 {
        *self as u64
    }
}

#[derive(Debug, Clone)]
struct Chunk<K, V> {
    map: FxMap<K, V>,
    max_end: u32,
}

/// A hash map split into `2^bits` copy-on-write chunks behind a
/// copy-on-write chunk table.
#[derive(Debug, Clone)]
pub(crate) struct CowMap<K, V> {
    chunks: Rc<Vec<Rc<Chunk<K, V>>>>,
    bits: u32,
}

impl<K: ChunkKey, V: Clone + SpanBound> CowMap<K, V> {
    fn new(bits: u32) -> Self {
        let chunk = Rc::new(Chunk { map: FxMap::default(), max_end: 0 });
        CowMap { chunks: Rc::new(vec![chunk; 1 << bits]), bits }
    }

    #[inline]
    fn index(&self, key: &K) -> usize {
        if self.bits == 0 {
            0
        } else {
            (key.chunk_hash().wrapping_mul(0x9E37_79B9_7F4A_7C15) >> (64 - self.bits)) as usize
        }
    }

    #[inline]
    fn get(&self, key: &K) -> Option<&V> {
        self.chunks[self.index(key)].map.get(key)
    }

    fn insert(&mut self, key: K, value: V) {
        let i = self.index(&key);
        let end = value.max_end();
        let chunk = Rc::make_mut(&mut Rc::make_mut(&mut self.chunks)[i]);
        chunk.max_end = chunk.max_end.max(end);
        chunk.map.insert(key, value);
    }

    fn values(&self) -> impl Iterator<Item = &V> {
        self.chunks.iter().flat_map(|c| c.map.values())
    }

    /// `f` applied to every value; `None` if `f` rejects one. Chunks whose
    /// bound is at most `identity_bound` (spans `f` cannot change) are shared.
    fn map_values(&self, f: &dyn Fn(&V) -> Option<V>, identity_bound: u32) -> Option<Self> {
        let mut chunks = Vec::with_capacity(self.chunks.len());
        for c in self.chunks.iter() {
            if c.max_end <= identity_bound {
                chunks.push(c.clone());
                continue;
            }
            let mut map = FxMap::with_capacity_and_hasher(c.map.len(), Default::default());
            let mut max_end = 0;
            for (k, v) in &c.map {
                let v = f(v)?;
                max_end = max_end.max(v.max_end());
                map.insert(k.clone(), v);
            }
            chunks.push(Rc::new(Chunk { map, max_end }));
        }
        Some(CowMap { chunks: Rc::new(chunks), bits: self.bits })
    }

    /// Does `old` with every value passed through the span mapping equal
    /// `new`? `eq(old_value, new_value)` compares one pair modulo the mapping.
    fn eq_mapped(old: &Self, new: &Self, eq: &dyn Fn(&V, &V) -> bool, identity_bound: u32) -> bool {
        if old.bits != new.bits {
            return false;
        }
        let whole = Rc::ptr_eq(&old.chunks, &new.chunks);
        for (i, o) in old.chunks.iter().enumerate() {
            let n = &new.chunks[i];
            if (whole || Rc::ptr_eq(o, n)) && o.max_end <= identity_bound {
                continue;
            }
            if o.map.len() != n.map.len() {
                return false;
            }
            for (k, v) in &o.map {
                match n.map.get(k) {
                    Some(nv) if eq(v, nv) => {}
                    _ => return false,
                }
            }
        }
        true
    }
}

impl<K: ChunkKey, V: Clone + SpanBound + PartialEq> PartialEq for CowMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        CowMap::eq_mapped(self, other, &|a, b| a == b, u32::MAX)
    }
}

/// The group save stack: frames behind a copy-on-write table, each frame
/// copy-on-write itself (only the innermost one is normally written).
#[derive(Debug, Clone, PartialEq)]
struct Frames(Rc<Vec<Rc<Frame>>>);

impl Frames {
    fn last_mut(&mut self) -> &mut Frame {
        Rc::make_mut(Rc::make_mut(&mut self.0).last_mut().expect("the outermost frame is never popped"))
    }

    fn push_save(&mut self, item: SaveItem) {
        let end = item.max_end();
        let frame = self.last_mut();
        frame.max_end = frame.max_end.max(end);
        frame.saves.push(item);
    }

    /// Drop save entries matching `pred` from every frame but the outermost,
    /// copying only frames that actually hold one.
    fn retain_saves(&mut self, pred: impl Fn(&SaveItem) -> bool) {
        if !self.0.iter().skip(1).any(|f| f.saves.iter().any(|s| !pred(s))) {
            return;
        }
        for frame in Rc::make_mut(&mut self.0).iter_mut().skip(1) {
            if frame.saves.iter().any(|s| !pred(s)) {
                Rc::make_mut(frame).saves.retain(|s| pred(s));
            }
        }
    }
}

const INT_PARAMS: usize = 5;

fn int_param_index(p: IntParam) -> usize {
    match p {
        IntParam::Escapechar => 0,
        IntParam::Endlinechar => 1,
        IntParam::Newlinechar => 2,
        IntParam::ETeXVersion => 3,
        IntParam::Font => 4,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scopes {
    cs: CowMap<String, Meaning>,
    active: CowMap<char, Meaning>,
    cat_table: CatCodeTable,
    uccode: CowMap<char, char>,
    lccode: CowMap<char, char>,
    int_params: [i64; INT_PARAMS],
    count: CowMap<u16, i64>,
    dimen: CowMap<u16, i64>,
    skip: CowMap<u16, Glue>,
    toks: CowMap<u16, Vec<Token>>,
    frames: Frames,
}

impl Scopes {
    pub fn new() -> Self {
        let mut int_params = [0; INT_PARAMS];
        int_params[int_param_index(IntParam::Escapechar)] = '\\' as i64;
        int_params[int_param_index(IntParam::Endlinechar)] = 13;
        int_params[int_param_index(IntParam::Newlinechar)] = -1;
        int_params[int_param_index(IntParam::ETeXVersion)] = 2;
        Scopes {
            cs: CowMap::new(6),
            active: CowMap::new(0),
            cat_table: CatCodeTable::latex_initial(),
            uccode: CowMap::new(0),
            lccode: CowMap::new(0),
            int_params,
            count: CowMap::new(2),
            dimen: CowMap::new(2),
            skip: CowMap::new(1),
            toks: CowMap::new(1),
            frames: Frames(Rc::new(vec![Rc::new(Frame::new())])),
        }
    }

    // -- control sequences --------------------------------------------

    pub fn meaning(&self, name: &str) -> Meaning {
        self.meaning_ref(name).cloned().unwrap_or(Meaning::Undefined)
    }

    pub fn meaning_ref(&self, name: &str) -> Option<&Meaning> {
        // `CowMap::get` takes `&String`; hash the `&str` the same way.
        let chunk = &self.cs.chunks[self.cs_index(name)];
        chunk.map.get(name)
    }

    #[inline]
    fn cs_index(&self, name: &str) -> usize {
        let mut h = FxHasher::default();
        h.write(name.as_bytes());
        (h.finish().wrapping_mul(0x9E37_79B9_7F4A_7C15) >> (64 - self.cs.bits)) as usize
    }

    /// Is `name` currently defined (anything but `undefined`)? A group
    /// end can restore an entry *to* `Undefined`, so presence in the map
    /// is not enough (e-TeX `\ifcsname`, LaTeX `\@ifundefined`).
    pub fn is_defined(&self, name: &str) -> bool {
        !matches!(self.meaning_ref(name), None | Some(Meaning::Undefined))
    }

    pub fn active_meaning(&self, c: char) -> Meaning {
        self.active.get(&c).cloned().unwrap_or(Meaning::Undefined)
    }

    fn saving(&self) -> bool {
        // The outermost frame is never popped, so save entries there
        // would only ever accumulate; skip them.
        self.frames.0.len() > 1
    }

    pub fn assign_cs(&mut self, name: &str, meaning: Meaning, global: bool) {
        if !global && self.saving() {
            let previous = self.meaning(name);
            self.frames.push_save(SaveItem::CsMeaning(name.to_string(), previous));
        }
        if global {
            // TeX's `\global` assignment also wipes any pending local
            // save entries for the same name? No -- TeX keeps them
            // (eq_destroy), but their restoration at group end is
            // suppressed only when the *saved* level marks it global.
            // We model that by restoring nothing special: a later group
            // end restores the local pre-assignment value, exactly like
            // TeX's behaviour for `{\def\a{1}\global\def\a{2}}` where
            // `\a` is `2` afterwards because TeX's `unsave` skips
            // restoring entries whose current level is `level_one`.
            self.retain_global_marker(name);
        }
        self.cs.insert(name.to_string(), meaning);
    }

    /// TeX rule (tex.web §283, `unsave`): once a control sequence has
    /// been assigned `\global`ly, save-stack entries for it made *in the
    /// current group before* that global assignment are discarded rather
    /// than restored. We implement that by dropping those entries now.
    fn retain_global_marker(&mut self, name: &str) {
        self.frames.retain_saves(|s| !matches!(s, SaveItem::CsMeaning(n, _) if n == name));
    }

    pub fn assign_active(&mut self, c: char, meaning: Meaning, global: bool) {
        // Active characters share the same save mechanism, keyed under a
        // synthetic name so one Vec<SaveItem> variant suffices.
        let key = format!("~active~{c}");
        if !global && self.saving() {
            let previous = self.active_meaning(c);
            self.frames.push_save(SaveItem::CsMeaning(key, previous));
        } else {
            self.retain_global_marker(&key);
        }
        self.active.insert(c, meaning);
    }

    fn restore_cs_or_active(&mut self, key: String, meaning: Meaning) {
        if let Some(c) = key.strip_prefix("~active~").and_then(|s| s.chars().next()) {
            self.active.insert(c, meaning);
        } else {
            self.cs.insert(key, meaning);
        }
    }

    // -- catcodes / uccode / lccode / integer parameters -----------------

    pub fn catcode(&self, c: char) -> CatCode {
        self.cat_table.get(c)
    }

    pub fn set_catcode(&mut self, c: char, cat: CatCode, global: bool) {
        if !global && self.saving() {
            let old = self.cat_table.get(c);
            self.frames.push_save(SaveItem::Catcode(c, old));
        }
        self.cat_table.set(c, cat);
    }

    pub fn cat_table(&self) -> &CatCodeTable {
        &self.cat_table
    }

    /// `\uccode`: INITEX sets uccode of letters to their uppercase form
    /// and everything else to 0 (TeXbook p. 41).
    pub fn uccode(&self, c: char) -> char {
        if let Some(u) = self.uccode.get(&c) {
            return *u;
        }
        if c.is_ascii_lowercase() {
            c.to_ascii_uppercase()
        } else if c.is_ascii_uppercase() {
            c
        } else {
            '\0'
        }
    }

    pub fn lccode(&self, c: char) -> char {
        if let Some(l) = self.lccode.get(&c) {
            return *l;
        }
        if c.is_ascii_uppercase() {
            c.to_ascii_lowercase()
        } else if c.is_ascii_lowercase() {
            c
        } else {
            '\0'
        }
    }

    pub fn set_uccode(&mut self, c: char, v: char, global: bool) {
        if !global && self.saving() {
            let old = self.uccode(c);
            self.frames.push_save(SaveItem::Uccode(c, old));
        }
        self.uccode.insert(c, v);
    }

    pub fn set_lccode(&mut self, c: char, v: char, global: bool) {
        if !global && self.saving() {
            let old = self.lccode(c);
            self.frames.push_save(SaveItem::Lccode(c, old));
        }
        self.lccode.insert(c, v);
    }

    pub fn int_param(&self, p: IntParam) -> i64 {
        self.int_params[int_param_index(p)]
    }

    pub fn set_int_param(&mut self, p: IntParam, v: i64, global: bool) {
        if !global && self.saving() {
            let old = self.int_param(p);
            self.frames.push_save(SaveItem::IntParam(p, old));
        }
        self.int_params[int_param_index(p)] = v;
    }

    // -- registers ---------------------------------------------------------

    pub fn count(&self, idx: u16) -> i64 {
        *self.count.get(&idx).unwrap_or(&0)
    }
    pub fn set_count(&mut self, idx: u16, v: i64, global: bool) {
        if !global && self.saving() {
            let old = self.count(idx);
            self.frames.push_save(SaveItem::Count(idx, old));
        } else {
            self.frames.retain_saves(|s| !matches!(s, SaveItem::Count(i, _) if *i == idx));
        }
        self.count.insert(idx, v);
    }
    pub fn dimen(&self, idx: u16) -> i64 {
        *self.dimen.get(&idx).unwrap_or(&0)
    }
    pub fn set_dimen(&mut self, idx: u16, v: i64, global: bool) {
        if !global && self.saving() {
            let old = self.dimen(idx);
            self.frames.push_save(SaveItem::Dimen(idx, old));
        } else {
            self.frames.retain_saves(|s| !matches!(s, SaveItem::Dimen(i, _) if *i == idx));
        }
        self.dimen.insert(idx, v);
    }
    pub fn skip(&self, idx: u16) -> Glue {
        *self.skip.get(&idx).unwrap_or(&Glue::fixed(0))
    }
    pub fn set_skip(&mut self, idx: u16, v: Glue, global: bool) {
        if !global && self.saving() {
            let old = self.skip(idx);
            self.frames.push_save(SaveItem::Skip(idx, old));
        } else {
            self.frames.retain_saves(|s| !matches!(s, SaveItem::Skip(i, _) if *i == idx));
        }
        self.skip.insert(idx, v);
    }
    pub fn toks(&self, idx: u16) -> Vec<Token> {
        self.toks.get(&idx).cloned().unwrap_or_default()
    }
    pub fn set_toks(&mut self, idx: u16, v: Vec<Token>, global: bool) {
        if !global && self.saving() {
            let old = self.toks(idx);
            self.frames.push_save(SaveItem::Toks(idx, old));
        } else {
            self.frames.retain_saves(|s| !matches!(s, SaveItem::Toks(i, _) if *i == idx));
        }
        self.toks.insert(idx, v);
    }

    // -- grouping ------------------------------------------------------

    pub fn push_group(&mut self) {
        Rc::make_mut(&mut self.frames.0).push(Rc::new(Frame::new()));
    }

    pub fn pop_group(&mut self) -> Vec<Token> {
        if self.frames.0.len() <= 1 {
            return Vec::new();
        }
        let frame = Rc::make_mut(&mut self.frames.0).pop().unwrap();
        let frame = Rc::try_unwrap(frame).unwrap_or_else(|shared| (*shared).clone());
        for item in frame.saves.into_iter().rev() {
            match item {
                SaveItem::CsMeaning(name, m) => self.restore_cs_or_active(name, m),
                SaveItem::Catcode(c, cat) => self.cat_table.set(c, cat),
                SaveItem::Uccode(c, v) => self.uccode.insert(c, v),
                SaveItem::Lccode(c, v) => self.lccode.insert(c, v),
                SaveItem::IntParam(p, v) => self.int_params[int_param_index(p)] = v,
                SaveItem::Count(idx, v) => self.count.insert(idx, v),
                SaveItem::Dimen(idx, v) => self.dimen.insert(idx, v),
                SaveItem::Skip(idx, v) => self.skip.insert(idx, v),
                SaveItem::Toks(idx, v) => self.toks.insert(idx, v),
            }
        }
        frame.after_group
    }

    pub fn queue_aftergroup(&mut self, tok: Token) {
        let end = span_end(tok.span);
        let frame = self.frames.last_mut();
        frame.max_end = frame.max_end.max(end);
        frame.after_group.push(tok);
    }

    pub fn depth(&self) -> usize {
        self.frames.0.len()
    }

    /// Iterate every stored meaning (control sequences, active characters,
    /// saved meanings).
    pub fn for_each_meaning<'s>(&'s self, mut f: impl FnMut(&'s Meaning)) {
        for m in self.cs.values() {
            f(m);
        }
        for m in self.active.values() {
            f(m);
        }
        for frame in self.frames.0.iter() {
            for s in &frame.saves {
                if let SaveItem::CsMeaning(_, m) = s {
                    f(m);
                }
            }
        }
    }
}

impl Default for Scopes {
    fn default() -> Self {
        Self::new()
    }
}

// ---- span mapping (incremental convergence) ---------------------------

fn map_token(t: &Token, f: &dyn Fn(Span) -> Option<Span>) -> Option<Token> {
    Some(Token::new(t.kind.clone(), f(t.span)?))
}

fn map_tokens(ts: &[Token], f: &dyn Fn(Span) -> Option<Span>) -> Option<Vec<Token>> {
    ts.iter().map(|t| map_token(t, f)).collect()
}

fn map_macro(d: &MacroDef, f: &dyn Fn(Span) -> Option<Span>) -> Option<MacroDef> {
    use crate::macro_def::{BodyPart, ParamPart};
    let params = d
        .params
        .iter()
        .map(|p| match p {
            ParamPart::Literal(t) => map_token(t, f).map(ParamPart::Literal),
            ParamPart::Param(n) => Some(ParamPart::Param(*n)),
        })
        .collect::<Option<Vec<_>>>()?;
    let body = d
        .body
        .iter()
        .map(|p| match p {
            BodyPart::Literal(t) => map_token(t, f).map(BodyPart::Literal),
            BodyPart::Param(n) => Some(BodyPart::Param(*n)),
        })
        .collect::<Option<Vec<_>>>()?;
    Some(MacroDef { params, body, flags: d.flags, arity: d.arity })
}

pub(crate) fn map_meaning(m: &Meaning, f: &dyn Fn(Span) -> Option<Span>) -> Option<Meaning> {
    Some(match m {
        Meaning::Macro(d) => Meaning::Macro(Rc::new(map_macro(d, f)?)),
        Meaning::Let(inner) => Meaning::Let(Box::new(map_meaning(inner, f)?)),
        Meaning::CharLike(t) => Meaning::CharLike(map_token(t, f)?),
        other => other.clone(),
    })
}

/// `map_token(old, f) == Some(new)`, without building the mapped token.
#[inline]
pub(crate) fn token_eq_mapped(old: &Token, new: &Token, f: &dyn Fn(Span) -> Option<Span>) -> bool {
    old.kind == new.kind && f(old.span) == Some(new.span)
}

fn tokens_eq_mapped(old: &[Token], new: &[Token], f: &dyn Fn(Span) -> Option<Span>) -> bool {
    old.len() == new.len() && old.iter().zip(new).all(|(o, n)| token_eq_mapped(o, n, f))
}

/// `map_meaning(old, f) == Some(new)`, without building the mapped meaning.
fn meaning_eq_mapped(old: &Meaning, new: &Meaning, f: &dyn Fn(Span) -> Option<Span>) -> bool {
    use crate::macro_def::{BodyPart, ParamPart};
    match (old, new) {
        (Meaning::Macro(a), Meaning::Macro(b)) => {
            a.flags == b.flags
                && a.arity == b.arity
                && a.params.len() == b.params.len()
                && a.body.len() == b.body.len()
                && a.params.iter().zip(&b.params).all(|pair| match pair {
                    (ParamPart::Literal(o), ParamPart::Literal(n)) => token_eq_mapped(o, n, f),
                    (ParamPart::Param(o), ParamPart::Param(n)) => o == n,
                    _ => false,
                })
                && a.body.iter().zip(&b.body).all(|pair| match pair {
                    (BodyPart::Literal(o), BodyPart::Literal(n)) => token_eq_mapped(o, n, f),
                    (BodyPart::Param(o), BodyPart::Param(n)) => o == n,
                    _ => false,
                })
        }
        (Meaning::Let(a), Meaning::Let(b)) => meaning_eq_mapped(a, b, f),
        (Meaning::CharLike(a), Meaning::CharLike(b)) => token_eq_mapped(a, b, f),
        (Meaning::Macro(_) | Meaning::Let(_) | Meaning::CharLike(_), _) => false,
        (a, b) => a == b,
    }
}

fn save_eq_mapped(old: &SaveItem, new: &SaveItem, f: &dyn Fn(Span) -> Option<Span>) -> bool {
    match (old, new) {
        (SaveItem::CsMeaning(a, m), SaveItem::CsMeaning(b, n)) => a == b && meaning_eq_mapped(m, n, f),
        (SaveItem::Toks(a, v), SaveItem::Toks(b, w)) => a == b && tokens_eq_mapped(v, w, f),
        (SaveItem::CsMeaning(..) | SaveItem::Toks(..), _) => false,
        (a, b) => a == b,
    }
}

impl Scopes {
    /// Rebuild this state with every stored token span passed through
    /// `f`; `None` if any span is rejected. Used to compare an old
    /// checkpoint's state against a new run's after an edit shifted the
    /// source. `f` must leave every document span with `end <=
    /// identity_bound` unchanged; tables holding only such spans are shared,
    /// not copied.
    pub fn map_spans(&self, f: &dyn Fn(Span) -> Option<Span>, identity_bound: u32) -> Option<Scopes> {
        let meaning = |m: &Meaning| map_meaning(m, f);
        let cs = self.cs.map_values(&meaning, identity_bound)?;
        let active = self.active.map_values(&meaning, identity_bound)?;
        let toks = self.toks.map_values(&|v: &Vec<Token>| map_tokens(v, f), identity_bound)?;
        let frames = if self.frames.0.iter().all(|fr| fr.max_end <= identity_bound) {
            self.frames.clone()
        } else {
            let frames = self
                .frames
                .0
                .iter()
                .map(|fr| {
                    if fr.max_end <= identity_bound {
                        return Some(fr.clone());
                    }
                    let saves = fr
                        .saves
                        .iter()
                        .map(|s| {
                            Some(match s {
                                SaveItem::CsMeaning(n, m) => SaveItem::CsMeaning(n.clone(), map_meaning(m, f)?),
                                SaveItem::Toks(i, v) => SaveItem::Toks(*i, map_tokens(v, f)?),
                                other => other.clone(),
                            })
                        })
                        .collect::<Option<Vec<_>>>()?;
                    let after_group = map_tokens(&fr.after_group, f)?;
                    let max_end = saves.iter().map(SpanBound::max_end).max().unwrap_or(0).max(tokens_end(&after_group));
                    Some(Rc::new(Frame { saves, after_group, max_end }))
                })
                .collect::<Option<Vec<_>>>()?;
            Frames(Rc::new(frames))
        };
        Some(Scopes {
            cs,
            active,
            cat_table: self.cat_table.clone(),
            uccode: self.uccode.clone(),
            lccode: self.lccode.clone(),
            int_params: self.int_params,
            count: self.count.clone(),
            dimen: self.dimen.clone(),
            skip: self.skip.clone(),
            toks,
            frames,
        })
    }

    /// `self.map_spans(f, identity_bound) == Some(new.clone())`, computed
    /// without building the mapped state: tables that are the same
    /// allocation on both sides and hold only spans `f` leaves alone are
    /// skipped outright, so the cost follows what changed since the two
    /// states diverged.
    pub fn eq_mapped(&self, new: &Scopes, f: &dyn Fn(Span) -> Option<Span>, identity_bound: u32) -> bool {
        let Scopes { cs, active, cat_table, uccode, lccode, int_params, count, dimen, skip, toks, frames } = self;
        let meaning = |a: &Meaning, b: &Meaning| meaning_eq_mapped(a, b, f);
        int_params == &new.int_params
            && cat_table == &new.cat_table
            && frames.0.len() == new.frames.0.len()
            && count == &new.count
            && dimen == &new.dimen
            && skip == &new.skip
            && uccode == &new.uccode
            && lccode == &new.lccode
            && CowMap::eq_mapped(active, &new.active, &meaning, identity_bound)
            && CowMap::eq_mapped(toks, &new.toks, &|a: &Vec<Token>, b: &Vec<Token>| tokens_eq_mapped(a, b, f), identity_bound)
            && frames.0.iter().zip(new.frames.0.iter()).all(|(o, n)| {
                (Rc::ptr_eq(o, n) && o.max_end <= identity_bound)
                    || (o.saves.len() == n.saves.len()
                        && o.saves.iter().zip(&n.saves).all(|(a, b)| save_eq_mapped(a, b, f))
                        && tokens_eq_mapped(&o.after_group, &n.after_group, f))
            })
            && CowMap::eq_mapped(cs, &new.cs, &meaning, identity_bound)
    }
}
