//! Whether two engine states are the same up to where dynamic memory put
//! things (DESIGN.md §5.3: an address-independent comparison).
//!
//! The convergence test (`crate::incr`) compares the live state N with an
//! old run's checkpoint O. Where both runs allocated the same nodes at the
//! same places, the word spaces are equal but for free cells; where an edit
//! made the new run allocate differently (a paragraph with one more
//! character), every later node sits elsewhere and the comparison has to
//! relabel. [`Iso::check`] does that: it walks everything reachable from the
//! engine's roots in both states at once -- `eqtb`, the save stack parsed as
//! `show_save_groups` parses it, the semantic nest, the input stack, the
//! marks, the page builder's lists, the alignment and conditional stacks,
//! e-TeX's sparse arrays, pdfTeX's pending objects and lists -- pairing each
//! node of O with the node of N in the same place of the structure
//! (a bijection: a node is paired once), comparing every data field exactly
//! and following every pointer field. It then requires that every word that
//! differs between the two spaces lies in a node the walk compared (or in a
//! cell free in both), and that every pointer into the middle of a
//! structure (list tails, `loc`, `best_page_break`) points at paired nodes.
//!
//! The node formats are pdftex.web's, section by section: `flush_node_list`
//! and `copy_node_list` (§202, §206) list what every node type owns; the
//! whatsits are pdfTeX's (`Wipe out the whatsit node`); the roots are the
//! globals that hold pointers between two commands. [`Iso::walk_one`] walks
//! one state and says which allocated cells no root reaches: the test that
//! the root set is complete (`flashtex-host iserve` command `checkmem`).

use crate::arena::{ChunkDiff, CHUNK_BYTES, CHUNK_SHIFT};
use crate::generated::types::{in_state_record, list_state_record, obj_entry};
use crate::generated::Globals;
use crate::statediff::ScalarSlot;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Constants of this build (tex.web values with web2c's sizes). The eqtb
// region boundaries are literals in the generated `show_eqtb`; a unit test
// checks them there.
// ---------------------------------------------------------------------------

const NULL: i32 = 0;
const MEM_BOT: i32 = 0;
const MEM_TOP: i32 = crate::generated::consts::mem_max;
const LO_MEM_STAT_MAX: i32 = 19; // fil_neg_glue + glue_spec_size - 1
const EMPTY_FLAG: i32 = 268_435_455;

// the static heads (tex.web §162, pdftex.web)
const PAGE_INS_HEAD: i32 = MEM_TOP;
const CONTRIB_HEAD: i32 = MEM_TOP - 1;
const PAGE_HEAD: i32 = MEM_TOP - 2;
const TEMP_HEAD: i32 = MEM_TOP - 3;
const HOLD_HEAD: i32 = MEM_TOP - 4;
const ADJUST_HEAD: i32 = MEM_TOP - 5;
const ACTIVE: i32 = MEM_TOP - 7;
const ALIGN_HEAD: i32 = MEM_TOP - 8;
const END_SPAN: i32 = MEM_TOP - 9;
const OMIT_TEMPLATE: i32 = MEM_TOP - 10;
const NULL_LIST: i32 = MEM_TOP - 11;
const LIG_TRICK: i32 = MEM_TOP - 12;
const BACKUP_HEAD: i32 = MEM_TOP - 13;
const PRE_ADJUST_HEAD: i32 = MEM_TOP - 14;
const HI_MEM_STAT_MIN: i32 = MEM_TOP - 14;

// eqtb (positions as in pdftex.web; the Rust array is 0-based: index p-1)
pub(crate) const GLUE_BASE: i32 = crate::generated::consts::layout_glue_base;
pub(crate) const LOCAL_BASE: i32 = crate::generated::consts::layout_local_base;
pub(crate) const INT_BASE: i32 = crate::generated::consts::layout_int_base;
use crate::readset::EQTB_SIZE;

// command codes (pdftex.web §207-§210)
const TOKS_REGISTER: i32 = 71;
const REGISTER: i32 = 89;
const MAX_COMMAND: i32 = 103; // this build: call = 114, box_ref = 122 (generated eq_destroy)
const CALL: i32 = MAX_COMMAND + 11;
const LONG_OUTER_CALL: i32 = MAX_COMMAND + 14;
const GLUE_REF: i32 = MAX_COMMAND + 17;
const SHAPE_REF: i32 = MAX_COMMAND + 18;
const BOX_REF: i32 = MAX_COMMAND + 19;

// node types (§133-§160, pdftex.web, e-TeX)
const HLIST: i32 = 0;
const VLIST: i32 = 1;
const RULE: i32 = 2;
const INS: i32 = 3;
const MARK: i32 = 4;
const ADJUST: i32 = 5;
const LIGATURE: i32 = 6;
const DISC: i32 = 7;
const WHATSIT: i32 = 8;
const MATH: i32 = 9;
const GLUE: i32 = 10;
const KERN: i32 = 11;
const PENALTY: i32 = 12;
const UNSET: i32 = 13;
const STYLE: i32 = 14;
const CHOICE: i32 = 15;
const ORD_NOAD: i32 = 16;
const RADICAL_NOAD: i32 = 24;
const FRACTION_NOAD: i32 = 25;
const UNDER_NOAD: i32 = 26;
const ACCENT_NOAD: i32 = 28;
const VCENTER_NOAD: i32 = 29;
const LEFT_NOAD: i32 = 30;
const RIGHT_NOAD: i32 = 31;
const MARGIN_KERN: i32 = 40;

// math field types (§681)
const SUB_BOX: i32 = 2;
const SUB_MLIST: i32 = 3;

// whatsit subtypes (pdftex.web §1341 and pdfTeX's extensions)
const OPEN_NODE: i32 = 0;
const WRITE_NODE: i32 = 1;
const CLOSE_NODE: i32 = 2;
const SPECIAL_NODE: i32 = 3;
const LATESPECIAL_NODE: i32 = 4;
const LANGUAGE_NODE: i32 = 5;
const PDF_FIRST: i32 = 7;

/// [`Iso::check`]'s error when its `stop` said so.
pub const STOPPED: &str = "stopped for newer work";
/// Tasks between two questions to `stop` (a task is a node, a list, a token
/// list...: a few hundred nanoseconds each).
const STOP_EVERY: usize = 1024;
/// pdftex.web: `pdf_dest_fitr`, the destination type that sets its own
/// width, height and depth.
const PDF_DEST_FITR: i32 = 7;

// save stack (§268), groups (§269)
const RESTORE_OLD_VALUE: i32 = 0;
const RESTORE_ZERO: i32 = 1;
const INSERT_TOKEN: i32 = 2;
const LEVEL_BOUNDARY: i32 = 3;
const RESTORE_SA: i32 = 4;
const BOTTOM_LEVEL: i32 = 0;
const SIMPLE_GROUP: i32 = 1;
const HBOX_GROUP: i32 = 2;
const ADJUSTED_HBOX_GROUP: i32 = 3;
const VBOX_GROUP: i32 = 4;
const VTOP_GROUP: i32 = 5;
const ALIGN_GROUP: i32 = 6;
const NO_ALIGN_GROUP: i32 = 7;
const OUTPUT_GROUP: i32 = 8;
const MATH_GROUP: i32 = 9;
const DISC_GROUP: i32 = 10;
const INSERT_GROUP: i32 = 11;
const VCENTER_GROUP: i32 = 12;
const MATH_CHOICE_GROUP: i32 = 13;
const SEMI_SIMPLE_GROUP: i32 = 14;
const MATH_SHIFT_GROUP: i32 = 15;
const MATH_LEFT_GROUP: i32 = 16;

// modes (§211)
const VMODE: i32 = 1;
const HMODE: i32 = VMODE + MAX_COMMAND + 1;
const MMODE: i32 = HMODE + MAX_COMMAND + 1;

// input stack (§303, §307)
const TOKEN_LIST: i32 = 0;
const MACRO: i32 = 5;
const PARAMETER: i32 = 0;
const U_TEMPLATE: i32 = 1;
const V_TEMPLATE: i32 = 2;

// e-TeX sparse arrays
const INDEX_NODE_SIZE: i32 = 9;
const POINTER_NODE_SIZE: i32 = 2;
const WORD_NODE_SIZE: i32 = 3;
const MARK_CLASS_NODE_SIZE: i32 = 4;
const DIMEN_VAL_LIMIT: i32 = 0x20;
const MU_VAL_LIMIT: i32 = 0x40;
const BOX_VAL_LIMIT: i32 = 0x50;
const TOK_VAL_LIMIT: i32 = 0x60;
const MARK_VAL: i32 = 6;

// pdfTeX actions
const PDF_ACTION_SIZE: i32 = 4;
const PDF_ACTION_PAGE: i32 = 0;
const PDF_ACTION_USER: i32 = 3;

#[inline]
fn rh(w: u64) -> i32 {
    w as u32 as i32
}
#[inline]
fn lh(w: u64) -> i32 {
    (w >> 32) as u32 as i32
}
#[inline]
fn b0(w: u64) -> i32 {
    ((w >> 48) & 0xFFFF) as i32
}
#[inline]
fn b1(w: u64) -> i32 {
    ((w >> 32) & 0xFFFF) as i32
}
/// `.int`/`.sc`: the low half (the high half of such a word is left as it
/// was by `set_int`, so it is not part of the value).
#[inline]
fn int(w: u64) -> i32 {
    w as u32 as i32
}

// ---------------------------------------------------------------------------
// Reading the two states
// ---------------------------------------------------------------------------

/// A state's word space, read by byte offset.
trait Space {
    fn word(&self, off: usize) -> u64;
}

/// The live space.
struct Live<'a> {
    bytes: &'a [u8],
}

impl Space for Live<'_> {
    #[inline]
    fn word(&self, off: usize) -> u64 {
        u64::from_le_bytes(self.bytes[off..off + 8].try_into().unwrap())
    }
}

/// The old run's space at a checkpoint, through a chunk table.
struct Old<'a> {
    table: Vec<*const u64>,
    d: &'a ChunkDiff,
    g: &'a Globals,
}

impl Space for Old<'_> {
    #[inline]
    fn word(&self, off: usize) -> u64 {
        let c = off >> CHUNK_SHIFT;
        match self.table.get(c) {
            // SAFETY: the table holds chunk pointers of CHUNK_BYTES each,
            // valid while the arena and the branch are unchanged.
            Some(&p) if !p.is_null() => unsafe { *p.add((off & (CHUNK_BYTES - 1)) >> 3) },
            _ => self.d.old_word(&self.g.arena, off),
        }
    }
}

/// Byte offsets of the arrays and scalars the walk reads.
struct Layout {
    mem: usize,
    eqtb: usize,
    save_stack: usize,
    nest: usize,
    input_stack: usize,
    param_stack: usize,
    cur_mark: usize,
    font_glue: usize,
    hyph_list: usize,
    hyph_word: usize,
    disc_ptr: usize,
    sa_root: usize,
    if_stack: usize,
    pdf_mem: usize,
    obj_tab: usize,
    head_tab: usize,
    pdf_link_stack: usize,
    scalars: HashMap<&'static str, (usize, usize)>,
}

impl Layout {
    fn new(g: &Globals, slots: &[ScalarSlot]) -> Layout {
        let off = |n: &str| {
            g.arena
                .regions
                .iter()
                .find(|r| r.name == n)
                .map_or(usize::MAX, |r| r.off)
        };
        Layout {
            mem: off("mem"),
            eqtb: off("eqtb"),
            save_stack: off("save_stack"),
            nest: off("nest"),
            input_stack: off("input_stack"),
            param_stack: off("param_stack"),
            cur_mark: off("cur_mark"),
            font_glue: off("font_glue"),
            hyph_list: off("hyph_list"),
            hyph_word: off("hyph_word"),
            disc_ptr: off("disc_ptr"),
            sa_root: off("sa_root"),
            if_stack: off("if_stack"),
            pdf_mem: off("pdf_mem"),
            obj_tab: off("obj_tab"),
            head_tab: off("head_tab"),
            pdf_link_stack: off("pdf_link_stack"),
            scalars: slots.iter().map(|s| (s.name, (s.off, s.size))).collect(),
        }
    }
}

/// One of the two states.
struct St<'a> {
    sp: &'a dyn Space,
    l: &'a Layout,
    hi_mem_min: i32,
}

impl St<'_> {
    #[inline]
    fn mem(&self, p: i32) -> u64 {
        self.sp.word(self.l.mem + p as usize * 8)
    }
    /// A 4-byte scalar (or the first 4 bytes of a bigger one).
    fn sc(&self, name: &str) -> i32 {
        let Some(&(off, _)) = self.l.scalars.get(name) else {
            panic!("iso: no scalar {name}");
        };
        let w = self.sp.word(off & !7);
        (w >> ((off & 7) * 8)) as u32 as i32
    }
    /// The bytes of a scalar record (cur_list, cur_input).
    fn sc_bytes(&self, name: &str) -> Vec<u8> {
        let Some(&(off, size)) = self.l.scalars.get(name) else {
            panic!("iso: no scalar {name}");
        };
        let mut v = Vec::with_capacity(size + 8);
        let mut o = off & !7;
        while o < off + size {
            v.extend_from_slice(&self.sp.word(o).to_le_bytes());
            o += 8;
        }
        v[off & 7..(off & 7) + size].to_vec()
    }
    fn i32_at(&self, base: usize, index: usize) -> i32 {
        let off = base + index * 4;
        let w = self.sp.word(off & !7);
        (w >> ((off & 7) * 8)) as u32 as i32
    }
    fn eqtb(&self, p: i32) -> u64 {
        self.sp.word(self.l.eqtb + (p as usize - 1) * 8)
    }
    fn save(&self, k: i32) -> u64 {
        self.sp.word(self.l.save_stack + k as usize * 8)
    }
    fn bytes_at(&self, off: usize, size: usize) -> Vec<u8> {
        let mut v = Vec::with_capacity(size + 16);
        let mut o = off & !7;
        while o < off + size {
            v.extend_from_slice(&self.sp.word(o).to_le_bytes());
            o += 8;
        }
        v[off & 7..(off & 7) + size].to_vec()
    }
    fn nest(&self, k: i32) -> list_state_record {
        let size = std::mem::size_of::<list_state_record>();
        rec_from(&self.bytes_at(self.l.nest + k as usize * size, size))
    }
    fn input(&self, k: i32) -> in_state_record {
        let size = std::mem::size_of::<in_state_record>();
        rec_from(&self.bytes_at(self.l.input_stack + k as usize * size, size))
    }
    fn obj(&self, k: i32) -> obj_entry {
        let size = std::mem::size_of::<obj_entry>();
        rec_from(&self.bytes_at(self.l.obj_tab + k as usize * size, size))
    }
    fn is_char(&self, p: i32) -> bool {
        p >= self.hi_mem_min
    }
}

/// A plain-data record from its bytes.
fn rec_from<T: Copy>(b: &[u8]) -> T {
    assert_eq!(b.len(), std::mem::size_of::<T>());
    // SAFETY: `T` is one of the generated plain-old-data records (integers
    // only), for which any bit pattern is a value.
    unsafe { std::ptr::read_unaligned(b.as_ptr() as *const T) }
}

// ---------------------------------------------------------------------------
// The walk
// ---------------------------------------------------------------------------

/// What a pointer points at.
#[derive(Clone, Copy, Debug)]
enum K {
    /// A node list (char nodes and variable-size nodes, by `link`).
    List,
    /// One node (not followed by its `link`: a box register's box, a
    /// leader, `incompleat_noad`).
    Node,
    /// A token list with a reference count (macro bodies, `\toks`, marks,
    /// `\write` texts): the count, then the tokens.
    Tok,
    /// Tokens without a reference-count node (macro parameters, backed-up
    /// lists): a chain of one-word nodes.
    Chain,
    Glue,
    /// A `\parshape` or e-TeX penalty array: `2 info(p) + 1` words.
    Shape,
    Action,
    /// e-TeX sparse array: an index node of the given level (1: the root,
    /// 4: the parents of the elements).
    SaIdx(u8),
    /// e-TeX sparse array element (register value or mark class).
    SaLeaf,
    /// A saved sparse-array item (the `sa_chain` list).
    SaChain,
    Cond,
    AlignStack,
    Pseudo,
    Lr,
    Hyph,
    Head,
}

pub struct Iso<'a> {
    o: St<'a>,
    n: St<'a>,
    /// Paired node heads: visited bits per state, and the partner of each
    /// relocated O node.
    head_o: Vec<u64>,
    head_n: Vec<u64>,
    fwd: HashMap<i32, i32>,
    /// Cells of the nodes compared, per state.
    cov_o: Vec<u64>,
    cov_n: Vec<u64>,
    todo: Vec<(K, i32, i32)>,
    /// Pointers into structures, checked once the walk is done.
    deferred: Vec<(&'static str, i32, i32)>,
    /// Pointers to noad fields (the math_group's saved field).
    deferred_fields: Vec<(i32, i32)>,
    pub nodes: usize,
    err: Option<String>,
    /// Entries of `hyph_list`.
    hyph_len: usize,
    /// Debugging: report every pairing of this node, with the task.
    watch: i32,
    /// Nothing from here on reads the dimensions of a destination other
    /// than `fitr` (see `whatsit`).
    dest_dims_dead: bool,
    /// Asked every [`STOP_EVERY`] tasks: newer work stops the walk
    /// ([`STOPPED`]).
    stop: Option<&'a mut dyn FnMut() -> bool>,
    steps: usize,
    cur_task: Option<(K, i32, i32)>,
}

fn bit(v: &[u64], p: i32) -> bool {
    v[p as usize >> 6] >> (p as usize & 63) & 1 == 1
}
fn set(v: &mut [u64], p: i32) {
    v[p as usize >> 6] |= 1 << (p as usize & 63)
}

macro_rules! fail {
    ($s:expr, $($a:tt)*) => {{
        if $s.err.is_none() {
            $s.err = Some(format!("{} (walking {:?})", format!($($a)*), $s.cur_task));
        }
        return;
    }};
}

impl<'a> Iso<'a> {
    fn new(o: St<'a>, n: St<'a>) -> Iso<'a> {
        let words = (MEM_TOP as usize + 64) / 64 + 1;
        Iso {
            o,
            n,
            head_o: vec![0; words],
            head_n: vec![0; words],
            fwd: HashMap::new(),
            cov_o: vec![0; words],
            cov_n: vec![0; words],
            todo: Vec::new(),
            deferred: Vec::new(),
            deferred_fields: Vec::new(),
            nodes: 0,
            err: None,
            hyph_len: 0,
            watch: std::env::var("FLASHTEX_ISO_WATCH")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(-1),
            cur_task: None,
            dest_dims_dead: false,
            stop: None,
            steps: 0,
        }
    }

    fn in_mem(p: i32) -> bool {
        (MEM_BOT..=MEM_TOP).contains(&p)
    }

    /// Pair node `a` of O with `b` of N: `Some(true)` the first time,
    /// `Some(false)` if they were paired already, `None` on a conflict.
    fn pair(&mut self, a: i32, b: i32) -> Option<bool> {
        if !Self::in_mem(a) || !Self::in_mem(b) {
            return None;
        }
        if self.watch == a {
            eprintln!(
                "[iso] watch {a}: paired during {:?} (already: {})",
                self.cur_task,
                bit(&self.head_o, a)
            );
        }
        if bit(&self.head_o, a) {
            let partner = self.fwd.get(&a).copied().unwrap_or(a);
            return (partner == b).then_some(false);
        }
        if bit(&self.head_n, b) {
            return None;
        }
        set(&mut self.head_o, a);
        set(&mut self.head_n, b);
        if a != b {
            self.fwd.insert(a, b);
        }
        self.nodes += 1;
        Some(true)
    }

    fn partner(&self, a: i32) -> Option<i32> {
        bit(&self.head_o, a).then(|| self.fwd.get(&a).copied().unwrap_or(a))
    }

    fn cover(&mut self, a: i32, b: i32, size: i32) {
        for k in 0..size {
            if Self::in_mem(a + k) {
                set(&mut self.cov_o, a + k);
            }
            if Self::in_mem(b + k) {
                set(&mut self.cov_n, b + k);
            }
        }
    }

    /// Follow pointer fields `a` (O) and `b` (N) to structures of kind `k`.
    fn ptr(&mut self, k: K, a: i32, b: i32) {
        if a == NULL && b == NULL {
            return;
        }
        if a == NULL || b == NULL {
            fail!(self, "{k:?} pointer null in one state only ({a}, {b})");
        }
        self.todo.push((k, a, b));
    }

    /// A pointer into a structure the walk pairs elsewhere.
    fn later(&mut self, what: &'static str, a: i32, b: i32) {
        if a == NULL && b == NULL {
            return;
        }
        self.deferred.push((what, a, b));
    }

    fn eq(&mut self, what: &str, x: i32, y: i32) {
        if x != y {
            fail!(self, "{what} differs ({x} vs {y})");
        }
    }

    fn run(&mut self) {
        while let Some((k, a, b)) = self.todo.pop() {
            if self.err.is_some() {
                return;
            }
            self.steps += 1;
            if self.steps.is_multiple_of(STOP_EVERY) {
                if let Some(stop) = self.stop.as_mut() {
                    if stop() {
                        self.err = Some(STOPPED.into());
                        return;
                    }
                }
            }
            self.cur_task = Some((k, a, b));
            match k {
                K::List => self.list(a, b),
                K::Node => self.node(a, b, false),
                K::Tok => self.tok(a, b),
                K::Chain => self.chain(a, b),
                K::Glue => self.glue(a, b),
                K::Shape => self.shape(a, b),
                K::Action => self.action(a, b),
                K::SaIdx(level) => self.sa_index(a, b, level),
                K::SaLeaf => self.sa_leaf(a, b),
                K::SaChain => self.sa_chain(a, b),
                K::Cond => self.cond(a, b),
                K::AlignStack => self.align_stack(a, b),
                K::Pseudo => self.pseudo(a, b),
                K::Lr => self.lr(a, b),
                K::Hyph => self.hyph(a, b),
                K::Head => self.head(a, b),
            }
        }
    }

    // ---- node lists -----------------------------------------------------

    fn list(&mut self, mut a: i32, mut b: i32) {
        loop {
            if a == NULL && b == NULL {
                return;
            }
            if a == NULL || b == NULL {
                fail!(self, "a list is longer in one state");
            }
            match self.pair(a, b) {
                None => fail!(self, "list node {a}/{b} paired inconsistently"),
                Some(false) => return, // the rest was walked already
                Some(true) => {}
            }
            self.node(a, b, true);
            if self.err.is_some() {
                return;
            }
            a = rh(self.o.mem(a));
            b = rh(self.n.mem(b));
        }
    }

    /// Compare node `a` of O with node `b` of N (already paired when
    /// `paired`) and queue what it points to.
    fn node(&mut self, a: i32, b: i32, paired: bool) {
        if !paired {
            match self.pair(a, b) {
                None => fail!(self, "node {a}/{b} paired inconsistently"),
                Some(false) => return,
                Some(true) => {}
            }
        }
        let (ca, cb) = (self.o.is_char(a), self.n.is_char(b));
        if ca != cb {
            fail!(self, "a char node in one state only");
        }
        let w0o = self.o.mem(a);
        let w0n = self.n.mem(b);
        if ca {
            // font and character
            self.cover(a, b, 1);
            self.eq("char node", lh(w0o), lh(w0n));
            return;
        }
        let (t, st) = (b0(w0o), b1(w0o));
        self.eq("node type", t, b0(w0n));
        self.eq("node subtype", st, b1(w0n));
        if self.err.is_some() {
            return;
        }
        let w = |s: &Self, k: i32| (s.o.mem(a + k), s.n.mem(b + k));
        match t {
            HLIST | VLIST | UNSET => {
                self.cover(a, b, 7);
                for k in 1..=4 {
                    let (x, y) = w(self, k);
                    self.eq("box dimension", int(x), int(y));
                }
                let (x, y) = w(self, 5);
                self.eq("glue order/sign", lh(x), lh(y));
                self.ptr(K::List, rh(x), rh(y));
                let (x, y) = w(self, 6);
                if t == UNSET {
                    self.eq("glue stretch", int(x), int(y));
                } else if x != y {
                    fail!(self, "glue set differs");
                }
            }
            RULE => {
                self.cover(a, b, 4);
                for k in 1..=3 {
                    let (x, y) = w(self, k);
                    self.eq("rule dimension", int(x), int(y));
                }
            }
            INS => {
                self.cover(a, b, 5);
                for k in 1..=3 {
                    let (x, y) = w(self, k);
                    self.eq("ins field", int(x), int(y));
                }
                let (x, y) = w(self, 4);
                self.ptr(K::List, lh(x), lh(y));
                self.ptr(K::Glue, rh(x), rh(y));
            }
            MARK => {
                self.cover(a, b, 2);
                let (x, y) = w(self, 1);
                self.eq("mark class", lh(x), lh(y));
                self.ptr(K::Tok, rh(x), rh(y));
            }
            ADJUST => {
                self.cover(a, b, 2);
                let (x, y) = w(self, 1);
                self.ptr(K::List, int(x), int(y));
            }
            LIGATURE => {
                self.cover(a, b, 2);
                let (x, y) = w(self, 1);
                self.eq("ligature char", lh(x), lh(y));
                self.ptr(K::List, rh(x), rh(y));
            }
            DISC => {
                self.cover(a, b, 2);
                let (x, y) = w(self, 1);
                self.ptr(K::List, lh(x), lh(y));
                self.ptr(K::List, rh(x), rh(y));
            }
            WHATSIT => self.whatsit(a, b, st),
            MATH | KERN | PENALTY => {
                self.cover(a, b, 2);
                let (x, y) = w(self, 1);
                self.eq("width/penalty", int(x), int(y));
            }
            GLUE => {
                self.cover(a, b, 2);
                let (x, y) = w(self, 1);
                self.ptr(K::Glue, lh(x), lh(y));
                self.ptr(K::Node, rh(x), rh(y));
            }
            MARGIN_KERN => {
                self.cover(a, b, 3);
                let (x, y) = w(self, 1);
                self.eq("margin kern", int(x), int(y));
                let (x, y) = w(self, 2);
                self.ptr(K::Node, lh(x), lh(y));
            }
            STYLE => {
                self.cover(a, b, 3);
                for k in 1..=2 {
                    let (x, y) = w(self, k);
                    self.eq("style node", int(x), int(y));
                }
            }
            CHOICE => {
                self.cover(a, b, 3);
                for k in 1..=2 {
                    let (x, y) = w(self, k);
                    self.ptr(K::List, lh(x), lh(y));
                    self.ptr(K::List, rh(x), rh(y));
                }
            }
            ORD_NOAD..=RIGHT_NOAD => self.noad(a, b, t),
            _ => fail!(self, "unknown node type {t} at {a}/{b}"),
        }
    }

    /// A math field (§681): `math_type` in `link`; a pointer in `info` for
    /// `sub_box`/`sub_mlist`, else a family and character.
    fn field(&mut self, x: u64, y: u64) {
        let (mt, nt) = (rh(x), rh(y));
        self.eq("math type", mt, nt);
        if mt == SUB_BOX || mt == SUB_MLIST {
            self.ptr(K::List, lh(x), lh(y));
        } else {
            self.eq("math char", lh(x), lh(y));
        }
    }

    fn noad(&mut self, a: i32, b: i32, t: i32) {
        let size = match t {
            RADICAL_NOAD | ACCENT_NOAD => 5,
            FRACTION_NOAD => 6,
            _ => 4,
        };
        self.cover(a, b, size);
        let w = |s: &Self, k: i32| (s.o.mem(a + k), s.n.mem(b + k));
        match t {
            FRACTION_NOAD => {
                let (x, y) = w(self, 1);
                self.eq("fraction thickness", int(x), int(y));
                for k in 2..=3 {
                    let (x, y) = w(self, k);
                    self.field(x, y);
                }
                for k in 4..=5 {
                    let (x, y) = w(self, k);
                    if x != y {
                        fail!(self, "fraction delimiter differs");
                    }
                }
            }
            LEFT_NOAD | RIGHT_NOAD => {
                // delimiter in the nucleus word (four quarterwords)
                for k in 1..=3 {
                    let (x, y) = w(self, k);
                    if x != y {
                        fail!(self, "left/right noad differs");
                    }
                }
            }
            _ => {
                for k in 1..=3 {
                    let (x, y) = w(self, k);
                    self.field(x, y);
                }
                if t == RADICAL_NOAD {
                    let (x, y) = w(self, 4);
                    if x != y {
                        fail!(self, "radical delimiter differs");
                    }
                } else if t == ACCENT_NOAD {
                    let (x, y) = w(self, 4);
                    self.field(x, y);
                }
                let _ = (UNDER_NOAD, VCENTER_NOAD);
            }
        }
    }

    fn whatsit(&mut self, a: i32, b: i32, st: i32) {
        let w = |s: &Self, k: i32| (s.o.mem(a + k), s.n.mem(b + k));
        let data = |s: &mut Self, from: i32, to: i32| {
            for k in from..=to {
                let (x, y) = (s.o.mem(a + k), s.n.mem(b + k));
                if (int(x) != int(y) || lh(x) != lh(y)) && s.err.is_none() {
                    s.err = Some(format!(
                        "whatsit {st} word {k} differs ({:#x} at {a} vs {:#x} at {b}; walking {:?})",
                        x, y, s.cur_task
                    ));
                }
            }
        };
        // Words pdftex.web accesses only as `.sc`/`.int` (the low half; the
        // high half is whatever the memory held, and it differs between
        // runs that allocated differently).
        let ints = |s: &mut Self, from: i32, to: i32| {
            for k in from..=to {
                let (x, y) = (s.o.mem(a + k), s.n.mem(b + k));
                if int(x) != int(y) && s.err.is_none() {
                    s.err = Some(format!(
                        "whatsit {st} word {k} differs ({:#x} at {a} vs {:#x} at {b}; walking {:?})",
                        x, y, s.cur_task
                    ));
                }
            }
        };
        match st {
            OPEN_NODE => {
                self.cover(a, b, 3);
                data(self, 1, 2);
            }
            WRITE_NODE | SPECIAL_NODE | LATESPECIAL_NODE => {
                self.cover(a, b, 2);
                let (x, y) = w(self, 1);
                self.eq("write stream", lh(x), lh(y));
                self.ptr(K::Tok, rh(x), rh(y));
            }
            CLOSE_NODE | LANGUAGE_NODE => {
                self.cover(a, b, 2);
                data(self, 1, 1);
            }
            _ => {
                let s = st - PDF_FIRST;
                match s {
                    // literal, lateliteral: data (link), mode (info)
                    0 | 1 => {
                        self.cover(a, b, 2);
                        let (x, y) = w(self, 1);
                        self.eq("literal mode", lh(x), lh(y));
                        self.ptr(K::Tok, rh(x), rh(y));
                    }
                    // refobj: `pdf_obj_objnum` is `info(p+1)` (the other half
                    // of the word is never set)
                    3 => {
                        self.cover(a, b, 2);
                        let (x, y) = w(self, 1);
                        self.eq("refobj objnum", lh(x), lh(y));
                    }
                    // refxform, refximage: width, height, depth (`.sc`),
                    // the object number in `info(p+4)`
                    5 | 7 => {
                        self.cover(a, b, 5);
                        ints(self, 1, 3);
                        let (x, y) = w(self, 4);
                        self.eq("xform/ximage objnum", lh(x), lh(y));
                    }
                    // annot: data in info(p+5), objnum in p+6. Words 1-3
                    // are `\pdfannot`'s width, height and depth (`.sc`);
                    // word 4 (`pdf_bottom`) is set only by `set_rect_dimens`
                    // at shipout, which writes it before any read.
                    8 => {
                        self.cover(a, b, 7);
                        ints(self, 1, 3);
                        let (x, y) = w(self, 5);
                        self.eq("annot word 5", rh(x), rh(y));
                        self.ptr(K::Tok, lh(x), lh(y));
                        ints(self, 6, 6);
                    }
                    // start_link: attr (info), action (link) in p+5; the
                    // rest as an annot's (`pdf_link_objnum` is `.int`)
                    9 => {
                        self.cover(a, b, 7);
                        ints(self, 1, 3);
                        let (x, y) = w(self, 5);
                        self.ptr(K::Tok, lh(x), lh(y));
                        self.ptr(K::Action, rh(x), rh(y));
                        ints(self, 6, 6);
                    }
                    // snapy_comp: `snapy_comp_ratio` (`.int`)
                    31 => {
                        self.cover(a, b, 2);
                        ints(self, 1, 1);
                    }
                    // end_link, end_thread, save_pos, snap_ref_point,
                    // interword_space on/off, fake_space, running_link
                    // off/on, save, restore: `small_node_size` nodes whose
                    // word 1 pdftex.web never sets or reads (it is copied
                    // with the node, as it is)
                    10 | 15 | 16 | 29 | 35 | 36 | 38 | 39 | 40 | 41 | 42 => {
                        self.cover(a, b, 2);
                    }
                    // dest: type/named_id/id in p+5, zoom/objnum in p+6
                    12 => {
                        self.cover(a, b, 7);
                        // Words 1-4 are `pdf_left` .. `pdf_bottom`, which
                        // `do_dest` writes when the page ships out
                        // (pdftex.web). Before that `\pdfdest` has set only
                        // a `fitr`'s `pdf_width`, `pdf_height` and
                        // `pdf_depth` (words 1-3); the rest holds whatever
                        // the memory held before `get_node` (the runs
                        // allocate differently, so it differs). The one
                        // read of those unset words before `do_dest` writes
                        // them is `set_rect_dimens`'s, with a
                        // `\pdfsetmatrix` in effect; word 4 is never read
                        // before it is written.
                        let (x, y) = w(self, 5);
                        let dims = if b0(x) == PDF_DEST_FITR {
                            3
                        } else if self.dest_dims_dead {
                            0
                        } else {
                            3
                        };
                        if dims > 0 {
                            ints(self, 1, dims);
                        }
                        self.eq("dest type/named", lh(x), lh(y));
                        if b1(x) > 0 {
                            self.ptr(K::Tok, rh(x), rh(y));
                        } else {
                            self.eq("dest id", rh(x), rh(y));
                        }
                        let (x, y) = w(self, 6);
                        if x != y && (lh(x), rh(x)) != (lh(y), rh(y)) {
                            fail!(self, "dest word 6 differs");
                        }
                    }
                    // thread, start_thread: named/id in p+5, attr in info(p+6)
                    13 | 14 => {
                        self.cover(a, b, 7);
                        // (words 1-4 as an annot's)
                        ints(self, 1, 3);
                        let (x, y) = w(self, 5);
                        self.eq("thread named", lh(x), lh(y));
                        if b1(x) > 0 {
                            self.ptr(K::Tok, rh(x), rh(y));
                        } else {
                            self.eq("thread id", rh(x), rh(y));
                        }
                        let (x, y) = w(self, 6);
                        self.eq("thread word 6", rh(x), rh(y));
                        self.ptr(K::Tok, lh(x), lh(y));
                    }
                    // snapy: snap_glue_ptr (info p+1), final_skip (p+2)
                    30 => {
                        self.cover(a, b, 3);
                        let (x, y) = w(self, 1);
                        self.eq("snapy word 1", rh(x), rh(y));
                        self.ptr(K::Glue, lh(x), lh(y));
                        let (x, y) = w(self, 2);
                        self.eq("final skip", int(x), int(y));
                    }
                    // colorstack: stack (link p+1), cmd (info p+1), data
                    // (link p+2) for a setter
                    33 => {
                        let (x, y) = w(self, 1);
                        self.eq("colorstack word 1", lh(x), lh(y));
                        self.eq("colorstack stack", rh(x), rh(y));
                        if lh(x) <= 1 {
                            self.cover(a, b, 3);
                            let (x, y) = w(self, 2);
                            self.ptr(K::Tok, rh(x), rh(y));
                        } else {
                            self.cover(a, b, 2);
                        }
                    }
                    // setmatrix: data (link p+1)
                    34 => {
                        self.cover(a, b, 2);
                        let (x, y) = w(self, 1);
                        self.ptr(K::Tok, rh(x), rh(y));
                    }
                    _ => fail!(self, "unknown whatsit subtype {st}"),
                }
            }
        }
    }

    // ---- token lists and small structures -------------------------------

    fn tok(&mut self, a: i32, b: i32) {
        match self.pair(a, b) {
            None => fail!(self, "token list {a}/{b} paired inconsistently"),
            Some(false) => return,
            Some(true) => {}
        }
        self.cover(a, b, 1);
        let (x, y) = (self.o.mem(a), self.n.mem(b));
        self.eq("token list reference count", lh(x), lh(y));
        self.chain_from(rh(x), rh(y));
    }

    fn chain(&mut self, a: i32, b: i32) {
        self.chain_from(a, b)
    }

    fn chain_from(&mut self, mut a: i32, mut b: i32) {
        loop {
            if a == NULL && b == NULL {
                return;
            }
            if a == NULL || b == NULL {
                fail!(self, "a token list is longer in one state");
            }
            match self.pair(a, b) {
                None => fail!(self, "token {a}/{b} paired inconsistently"),
                Some(false) => return,
                Some(true) => {}
            }
            self.cover(a, b, 1);
            let (x, y) = (self.o.mem(a), self.n.mem(b));
            self.eq("token", lh(x), lh(y));
            a = rh(x);
            b = rh(y);
        }
    }

    fn glue(&mut self, a: i32, b: i32) {
        match self.pair(a, b) {
            None => fail!(self, "glue spec {a}/{b} paired inconsistently"),
            Some(false) => return,
            Some(true) => {}
        }
        self.cover(a, b, 4);
        let (x, y) = (self.o.mem(a), self.n.mem(b));
        if (lh(x), rh(x)) != (lh(y), rh(y)) {
            fail!(self, "glue spec orders or reference count differ");
        }
        for k in 1..=3 {
            let (x, y) = (self.o.mem(a + k), self.n.mem(b + k));
            self.eq("glue component", int(x), int(y));
        }
    }

    fn shape(&mut self, a: i32, b: i32) {
        match self.pair(a, b) {
            None => fail!(self, "shape {a}/{b} paired inconsistently"),
            Some(false) => return,
            Some(true) => {}
        }
        let (x, y) = (self.o.mem(a), self.n.mem(b));
        let n = lh(x);
        self.eq("shape size", n, lh(y));
        if !(0..=1 << 20).contains(&n) {
            fail!(self, "bad shape size {n}");
        }
        self.cover(a, b, 2 * n + 1);
        for k in 1..=2 * n {
            let (x, y) = (self.o.mem(a + k), self.n.mem(b + k));
            self.eq("shape entry", int(x), int(y));
        }
    }

    fn action(&mut self, a: i32, b: i32) {
        match self.pair(a, b) {
            None => fail!(self, "action {a}/{b} paired inconsistently"),
            Some(false) => return,
            Some(true) => {}
        }
        self.cover(a, b, PDF_ACTION_SIZE);
        let w = |s: &Self, k: i32| (s.o.mem(a + k), s.n.mem(b + k));
        let (x, y) = w(self, 0);
        let (ty, named) = (b0(x), b1(x));
        self.eq("action type/named", lh(x), lh(y));
        if ty != PDF_ACTION_USER && ty != PDF_ACTION_PAGE && named & 1 == 1 {
            self.ptr(K::Tok, rh(x), rh(y));
        } else {
            self.eq("action id", rh(x), rh(y));
        }
        let (x, y) = w(self, 1);
        self.eq("action new window", rh(x), rh(y));
        if ty == PDF_ACTION_USER {
            self.eq("action file", lh(x), lh(y));
        } else {
            self.ptr(K::Tok, lh(x), lh(y));
        }
        let (x, y) = w(self, 2);
        self.eq("action reference count", rh(x), rh(y));
        if ty == PDF_ACTION_USER || ty == PDF_ACTION_PAGE {
            self.ptr(K::Tok, lh(x), lh(y));
        } else {
            self.eq("action tokens", lh(x), lh(y));
        }
        let (x, y) = w(self, 3);
        if ty != PDF_ACTION_USER && named & 2 == 2 {
            self.ptr(K::Tok, rh(x), rh(y));
        } else {
            self.eq("action struct id", rh(x), rh(y));
        }
        // (`info(p+3)` is not a field: pdftex.web never sets or reads it)
    }

    /// e-TeX's sparse arrays (pdftex.web "sparse arrays"): four levels of
    /// index nodes (`find_sa_element`), each with 16 child pointers and its
    /// parent in `link`; the elements below the fourth.
    fn sa_index(&mut self, a: i32, b: i32, level: u8) {
        if std::env::var_os("FLASHTEX_ISO_TRACE").is_some() {
            eprintln!(
                "[iso] sa index {a} level {level}: {:?}",
                (1..9)
                    .map(|k| self.o.mem(a + k))
                    .map(|w| (lh(w), rh(w)))
                    .collect::<Vec<_>>()
            );
        }
        match self.pair(a, b) {
            None => fail!(self, "sparse array index {a}/{b} paired inconsistently"),
            Some(false) => return,
            Some(true) => {}
        }
        let (x, y) = (self.o.mem(a), self.n.mem(b));
        self.eq("sa index/used", lh(x), lh(y));
        self.later("sa parent", rh(x), rh(y));
        self.cover(a, b, INDEX_NODE_SIZE);
        let k = if level >= 4 {
            K::SaLeaf
        } else {
            K::SaIdx(level + 1)
        };
        for j in 1..INDEX_NODE_SIZE {
            let (x, y) = (self.o.mem(a + j), self.n.mem(b + j));
            self.ptr(k, lh(x), lh(y));
            self.ptr(k, rh(x), rh(y));
        }
    }

    /// An element: count and dimen registers are word nodes, skip, muskip,
    /// box and toks registers pointer nodes, mark classes four-word nodes.
    fn sa_leaf(&mut self, a: i32, b: i32) {
        if std::env::var_os("FLASHTEX_ISO_TRACE").is_some() {
            eprintln!("[iso] sa leaf {a}");
        }
        match self.pair(a, b) {
            None => fail!(self, "sparse array element {a}/{b} paired inconsistently"),
            Some(false) => return,
            Some(true) => {}
        }
        let w = |s: &Self, k: i32| (s.o.mem(a + k), s.n.mem(b + k));
        let (x, y) = w(self, 0);
        self.eq("sa index/level", lh(x), lh(y));
        self.later("sa parent", rh(x), rh(y));
        let i = b0(x);
        if i / 16 == MARK_VAL {
            self.cover(a, b, MARK_CLASS_NODE_SIZE);
            for k in 1..=3 {
                let (x, y) = w(self, k);
                self.ptr(K::Tok, lh(x), lh(y));
                if k < 3 {
                    self.ptr(K::Tok, rh(x), rh(y));
                } else {
                    self.eq("mark class word 3", rh(x), rh(y));
                }
            }
            return;
        }
        let (x, y) = w(self, 1);
        self.eq("sa reference count", lh(x), lh(y));
        if i < DIMEN_VAL_LIMIT {
            self.cover(a, b, WORD_NODE_SIZE);
            self.eq("sa register number", rh(x), rh(y));
            let (x, y) = w(self, 2);
            self.eq("sa value", int(x), int(y));
        } else {
            self.cover(a, b, POINTER_NODE_SIZE);
            let k = match i / 16 {
                2 | 3 => K::Glue,
                4 => K::Node,
                _ => K::Tok,
            };
            let _ = (MU_VAL_LIMIT, BOX_VAL_LIMIT);
            self.ptr(k, rh(x), rh(y));
        }
    }

    /// The `sa_chain` of saved items: saved copies whose `sa_loc` points at
    /// the array element and whose `link` is the next saved item.
    fn sa_chain(&mut self, mut a: i32, mut b: i32) {
        loop {
            if a == NULL && b == NULL {
                return;
            }
            if a == NULL || b == NULL {
                fail!(self, "an sa_chain is longer in one state");
            }
            match self.pair(a, b) {
                None => fail!(self, "saved sa item {a}/{b} paired inconsistently"),
                Some(false) => return,
                Some(true) => {}
            }
            let (x, y) = (self.o.mem(a), self.n.mem(b));
            self.eq("saved sa index/level", lh(x), lh(y));
            let i = b0(x);
            let (x1, y1) = (self.o.mem(a + 1), self.n.mem(b + 1));
            // sa_loc: the element (paired by the tree walk)
            self.later("sa_loc", lh(x1), lh(y1));
            if i < DIMEN_VAL_LIMIT {
                self.cover(a, b, WORD_NODE_SIZE);
                self.eq("saved sa num", rh(x1), rh(y1));
                let (x2, y2) = (self.o.mem(a + 2), self.n.mem(b + 2));
                self.eq("saved sa value", int(x2), int(y2));
            } else if i == TOK_VAL_LIMIT {
                // a zero count/dimen saved as a pointer node
                self.cover(a, b, POINTER_NODE_SIZE);
                self.eq("saved sa zero", rh(x1), rh(y1));
            } else {
                self.cover(a, b, POINTER_NODE_SIZE);
                let k = match i / 16 {
                    2 | 3 => K::Glue,
                    4 => K::Node,
                    _ => K::Tok,
                };
                self.ptr(k, rh(x1), rh(y1));
            }
            a = rh(x);
            b = rh(y);
        }
    }

    /// The conditional stack (§489): `link` the enclosing one, type
    /// `if_limit`, subtype `cur_if`, then `if_line`.
    fn cond(&mut self, mut a: i32, mut b: i32) {
        loop {
            if a == NULL && b == NULL {
                return;
            }
            if a == NULL || b == NULL {
                fail!(self, "the conditional stacks differ in depth");
            }
            match self.pair(a, b) {
                None => fail!(self, "conditional {a}/{b} paired inconsistently"),
                Some(false) => return,
                Some(true) => {}
            }
            self.cover(a, b, 2);
            let (x, y) = (self.o.mem(a), self.n.mem(b));
            self.eq("conditional", lh(x), lh(y));
            let (x1, y1) = (self.o.mem(a + 1), self.n.mem(b + 1));
            self.eq("if_line", int(x1), int(y1));
            a = rh(x);
            b = rh(y);
        }
    }

    /// The alignment stack (§772): saved `cur_align` (into the preamble),
    /// `preamble`, `cur_span`, `cur_loop`, `align_state`, `cur_head/tail`,
    /// `cur_pre_head/tail`.
    fn align_stack(&mut self, mut a: i32, mut b: i32) {
        loop {
            if a == NULL && b == NULL {
                return;
            }
            if a == NULL || b == NULL {
                fail!(self, "the alignment stacks differ in depth");
            }
            match self.pair(a, b) {
                None => fail!(self, "alignment {a}/{b} paired inconsistently"),
                Some(false) => return,
                Some(true) => {}
            }
            self.cover(a, b, 6);
            let w = |s: &Self, k: i32| (s.o.mem(a + k), s.n.mem(b + k));
            let (x, y) = w(self, 0);
            self.later("saved cur_align", lh(x), lh(y));
            let (x1, y1) = w(self, 1);
            self.ptr(K::List, lh(x1), lh(y1)); // the saved preamble
            self.later("saved cur_span", rh(x1), rh(y1));
            let (x2, y2) = w(self, 2);
            self.later("saved cur_loop", int(x2), int(y2));
            let (x3, y3) = w(self, 3);
            self.eq("saved align_state", int(x3), int(y3));
            for k in 4..=5 {
                let (x, y) = w(self, k);
                self.ptr(K::Head, lh(x), lh(y)); // cur_head, cur_pre_head
                self.later("saved tail", rh(x), rh(y));
            }
            a = rh(x);
            b = rh(y);
        }
    }

    /// e-TeX's pseudo files (\scantokens): `link` the next file, `info`
    /// the list of lines, each a node of `info` words.
    fn pseudo(&mut self, mut a: i32, mut b: i32) {
        loop {
            if a == NULL && b == NULL {
                return;
            }
            if a == NULL || b == NULL {
                fail!(self, "pseudo files differ in number");
            }
            match self.pair(a, b) {
                None => fail!(self, "pseudo file {a}/{b} paired inconsistently"),
                Some(false) => return,
                Some(true) => {}
            }
            self.cover(a, b, 1);
            let (x, y) = (self.o.mem(a), self.n.mem(b));
            // the lines: nodes of size info(l), linked by link(l)
            let (mut l, mut m) = (lh(x), lh(y));
            loop {
                if l == NULL && m == NULL {
                    break;
                }
                if l == NULL || m == NULL {
                    fail!(self, "pseudo lines differ in number");
                }
                match self.pair(l, m) {
                    None => fail!(self, "pseudo line paired inconsistently"),
                    Some(false) => break,
                    Some(true) => {}
                }
                let (lx, ly) = (self.o.mem(l), self.n.mem(m));
                let sz = lh(lx);
                self.eq("pseudo line size", sz, lh(ly));
                if !(1..1 << 20).contains(&sz) {
                    fail!(self, "bad pseudo line size");
                }
                self.cover(l, m, sz);
                for k in 1..sz {
                    if self.o.mem(l + k) != self.n.mem(m + k) {
                        fail!(self, "pseudo line text differs");
                    }
                }
                l = rh(lx);
                m = rh(ly);
            }
            a = rh(x);
            b = rh(y);
        }
    }

    /// TeXXeT's LR stack: one-word nodes, `info` the LR type.
    fn lr(&mut self, a: i32, b: i32) {
        self.chain_from(a, b)
    }

    /// A one-word list head (`get_avail`, its `info` unused) and the node
    /// list after it (an alignment's adjust lists).
    fn head(&mut self, a: i32, b: i32) {
        match self.pair(a, b) {
            None => fail!(self, "head {a}/{b} paired inconsistently"),
            Some(false) => return,
            Some(true) => {}
        }
        self.cover(a, b, 1);
        let (x, y) = (self.o.mem(a), self.n.mem(b));
        self.ptr(K::List, rh(x), rh(y));
    }

    /// A hyphenation exception list: one-word nodes, `info` a position.
    fn hyph(&mut self, a: i32, b: i32) {
        self.chain_from(a, b)
    }
}

// ---------------------------------------------------------------------------
// The roots
// ---------------------------------------------------------------------------

/// Scalars holding pointers between two commands, and what they point at.
/// (`List`: owned lists; the rest are deferred pointers into structures.)
const ROOT_SCALARS: &[(&str, Option<K>)] = &[
    ("cond_ptr", Some(K::Cond)),
    ("align_ptr", Some(K::AlignStack)),
    ("pseudo_files", Some(K::Pseudo)),
    ("LR_ptr", Some(K::Lr)),
    ("sa_chain", Some(K::SaChain)),
    // pdfTeX's token lists kept for the end
    ("pdf_info_toks", Some(K::Chain)),
    ("pdf_catalog_toks", Some(K::Chain)),
    ("pdf_names_toks", Some(K::Chain)),
    ("pdf_trailer_toks", Some(K::Chain)),
    ("pdf_trailer_id_toks", Some(K::Chain)),
    // pdfTeX's rule node for images in draft mode (a permanent node)
    ("alt_rule", Some(K::Node)),
    // pointers into lists walked from elsewhere
    ("page_tail", None),
    ("best_page_break", None),
    ("cur_align", None),
    ("cur_span", None),
    ("cur_loop", None),
    ("cur_tail", None),
    ("cur_pre_tail", None),
    ("cur_head", None),
    ("cur_pre_head", None),
    ("adjust_tail", None),
    ("pre_adjust_tail", None),
    ("last_glue", None),
];

/// Whether the structural comparison answers for scalar `n` (it reads and
/// compares it, it is dead between commands, or it only says where the
/// allocator puts the next node).
pub fn scalar_covered(n: &str) -> bool {
    ROOT_SCALARS.iter().any(|(r, _)| *r == n)
        || DEAD_SCALARS.contains(&n)
        || matches!(
            n,
            "avail"
                | "rover"
                | "lo_mem_max"
                | "hi_mem_min"
                | "mem_end"
                | "cur_list"
                | "cur_input"
                | "cur_head"
                | "cur_pre_head"
                // statistics only: printed in the `\tracingstats` memory
                // lines and the end-of-run memory accounting, which P-T1
                // reports separately (DESIGN §1.1, ruling N2); they differ
                // by the cells leaked before the restart point
                | "var_used"
                | "dyn_used"
        )
}

/// Whether scalar `n` holds a value only while a command runs
/// (`DEAD_SCALARS`).
pub fn dead_scalar(n: &str) -> bool {
    DEAD_SCALARS.contains(&n)
}

/// Scalars that hold pointers only while a command runs (tex.web: set
/// before every use), so their values between commands are not state.
const DEAD_SCALARS: &[&str] = &[
    // tex.web §494: set by `pass_text` when skipping begins, read only by
    // the "Incomplete \if" message of that skip
    "skip_line",
    // §305, §389, §473: set by `macro_call` and `scan_toks` before the scan
    // whose runaway message prints it
    "warning_index",
    // `line_break`'s scalars (§833, §847, §872), set in each call before
    // they are read (§834, §848, §874, §875)
    "minimum_demerits",
    "easy_line",
    "last_special_line",
    "first_width",
    "second_width",
    "first_indent",
    "second_indent",
    "fewest_demerits",
    "best_line",
    "actual_looseness",
    "line_diff",
    "temp_ptr",
    "prev_tail",
    "def_ref",
    "cur_box",
    "cur_mlist",
    "just_box",
    "passive",
    "printed_node",
    // tex.web §864: zeroed with `printed_node` before each pass's first
    // `try_break`, its only reader
    "pass_number",
    "prev_p",
    "first_p",
    "prev_char_p",
    "next_char_p",
    "try_prev_break",
    "prev_legal",
    "prev_prev_legal",
    "rejected_cur_p",
    "before_rejected_cur_p",
    "cur_p",
    "best_bet",
    "last_line_fill",
    "ha",
    "hb",
    "init_list",
    "lig_stack",
    "cur_q",
    "main_p",
    "cur_cs",
    "cur_cmd",
    "cur_chr",
    "cur_tok",
    "cur_val",
    "cur_val_level",
    "cur_ptr",
    "save_tail",
    "cur_l",
    "cur_r",
    "cur_f",
    "cur_c",
    "cur_i",
    "main_f",
    "main_i",
    "main_j",
    "main_k",
    "main_s",
    "bchar",
    "false_bchar",
    "cancel_boundary",
    "ins_disc",
    "ligature_present",
    "lft_hit",
    "rt_hit",
    "hn",
    "hf",
    "hyf_char",
    "hyf_bchar",
    "init_lig",
    "init_lft",
    "hyphen_passed",
    "pdf_stream_length",
    // pdfTeX's per-page lists: reset as `pdf_ship_out` begins ("Reset
    // resource lists"), flushed as it ends, not reset then
    "pdf_obj_list",
    "pdf_xform_list",
    "pdf_ximage_list",
    "pdf_annot_list",
    "pdf_link_list",
    "pdf_dest_list",
    "pdf_bead_list",
    "pdf_font_list",
    "pdf_append_list_arg",
    "ff",
    "last_tokens_string",
    "tmp_w",
];

impl<'a> Iso<'a> {
    /// Every root of both states, queued or compared.
    fn roots(&mut self) {
        let (o, n) = (&self.o, &self.n);
        // the static heads' links
        let heads = [
            (CONTRIB_HEAD, K::List),
            (PAGE_HEAD, K::List),
            (HOLD_HEAD, K::List),
            (ADJUST_HEAD, K::List),
            (PRE_ADJUST_HEAD, K::List),
            (ALIGN_HEAD, K::List),
            (TEMP_HEAD, K::List),
            (BACKUP_HEAD, K::Chain),
        ];
        let mut q = vec![];
        for (h, k) in heads {
            q.push((k, rh(o.mem(h)), rh(n.mem(h))));
        }
        let _ = (ACTIVE, END_SPAN, NULL_LIST, LIG_TRICK, HI_MEM_STAT_MIN);
        for (k, a, b) in q {
            self.ptr(k, a, b);
        }
        // the static words themselves compare equal apart from those links
        for p in HI_MEM_STAT_MIN..=MEM_TOP {
            let (x, y) = (self.o.mem(p), self.n.mem(p));
            if (p == OMIT_TEMPLATE || p == END_SPAN || p == NULL_LIST) && x != y {
                fail!(self, "static word {p} differs");
            }
        }
        self.cover_static();
        // the page insertions (§980): a circular list from page_ins_head
        self.page_ins();
        // eqtb
        self.eqtb();
        self.save_stack();
        self.nest();
        self.input();
        self.arrays();
        let in_align = self.o.sc("align_ptr") != NULL;
        self.eq(
            "align_ptr null",
            in_align as i32,
            (self.n.sc("align_ptr") != NULL) as i32,
        );
        let page_empty = self.o.sc("page_contents") == 0;
        self.eq(
            "page_contents",
            self.o.sc("page_contents"),
            self.n.sc("page_contents"),
        );
        for &(name, k) in ROOT_SCALARS {
            let (a, b) = (self.o.sc(name), self.n.sc(name));
            // alignment state is live only inside an alignment; the best
            // break only while the page has contents
            let dead = (!in_align
                && matches!(
                    name,
                    "cur_align"
                        | "cur_span"
                        | "cur_loop"
                        | "cur_tail"
                        | "cur_pre_tail"
                        | "cur_head"
                        | "cur_pre_head"
                ))
                || (page_empty && name == "best_page_break");
            if dead {
                continue;
            }
            if name == "cur_head" || name == "cur_pre_head" {
                self.ptr(K::Head, a, b);
                continue;
            }
            match k {
                Some(k) => self.ptr(k, a, b),
                None => {
                    if name == "last_glue" {
                        let max_halfword = EMPTY_FLAG;
                        if a == max_halfword || b == max_halfword {
                            self.eq("last_glue", a, b);
                        } else {
                            self.ptr(K::Glue, a, b);
                        }
                    } else {
                        self.later(name, a, b);
                    }
                }
            }
        }
    }

    fn cover_static(&mut self) {
        // the one-word dummy at lo_mem_max (tex.web §164)
        let (lo, ln) = (self.o.sc("lo_mem_max"), self.n.sc("lo_mem_max"));
        if Self::in_mem(lo) {
            set(&mut self.cov_o, lo);
        }
        if Self::in_mem(ln) {
            set(&mut self.cov_n, ln);
        }
        for p in MEM_BOT..=LO_MEM_STAT_MAX {
            set(&mut self.cov_o, p);
            set(&mut self.cov_n, p);
        }
        for p in HI_MEM_STAT_MIN..=MEM_TOP {
            set(&mut self.cov_o, p);
            set(&mut self.cov_n, p);
        }
        // the static glue specs: reference counts and values
        for g in (MEM_BOT..=LO_MEM_STAT_MAX).step_by(4) {
            let _ = self.pair(g, g);
            for k in 0..4 {
                let (x, y) = (self.o.mem(g + k), self.n.mem(g + k));
                if (lh(x), rh(x)) != (lh(y), rh(y)) {
                    fail!(self, "static glue {g} differs");
                }
            }
        }
    }

    fn page_ins(&mut self) {
        let (mut a, mut b) = (rh(self.o.mem(PAGE_INS_HEAD)), rh(self.n.mem(PAGE_INS_HEAD)));
        let _ = self.pair(PAGE_INS_HEAD, PAGE_INS_HEAD);
        let mut steps = 0;
        while a != PAGE_INS_HEAD || b != PAGE_INS_HEAD {
            if a == PAGE_INS_HEAD || b == PAGE_INS_HEAD || steps > 1 << 20 {
                fail!(self, "the page insertion lists differ");
            }
            steps += 1;
            match self.pair(a, b) {
                None => fail!(self, "page insertion {a}/{b} paired inconsistently"),
                Some(false) => fail!(self, "page insertion list loops"),
                Some(true) => {}
            }
            self.cover(a, b, 4);
            let (x, y) = (self.o.mem(a), self.n.mem(b));
            self.eq("page insertion type/box", lh(x), lh(y));
            let (x1, y1) = (self.o.mem(a + 1), self.n.mem(b + 1));
            self.later("broken_ptr", rh(x1), rh(y1));
            self.later("broken_ins", lh(x1), lh(y1));
            let (x2, y2) = (self.o.mem(a + 2), self.n.mem(b + 2));
            self.later("last_ins_ptr", rh(x2), rh(y2));
            self.later("best_ins_ptr", lh(x2), lh(y2));
            let (x3, y3) = (self.o.mem(a + 3), self.n.mem(b + 3));
            self.eq("page insertion height", int(x3), int(y3));
            a = rh(x);
            b = rh(y);
        }
    }

    /// An eqtb word below `int_base` (or one saved on the save stack for
    /// position `p`): eq_type, eq_level and equiv, the equiv followed by
    /// its eq_type (`eq_destroy`, §275 and e-TeX).
    fn eqtb_word(&mut self, x: u64, y: u64) {
        let (t, ty) = (b0(x), b0(y));
        self.eq("eq_type", t, ty);
        self.eq("eq_level", b1(x), b1(y));
        let (a, b) = (rh(x), rh(y));
        match t {
            CALL..=LONG_OUTER_CALL => self.ptr(K::Tok, a, b),
            GLUE_REF => self.ptr(K::Glue, a, b),
            SHAPE_REF => self.ptr(K::Shape, a, b),
            BOX_REF => self.ptr(K::Node, a, b),
            TOKS_REGISTER | REGISTER if !(MEM_BOT..=LO_MEM_STAT_MAX).contains(&a) => {
                self.later("sparse register", a, b)
            }
            _ => self.eq("equiv", a, b),
        }
    }

    fn eqtb(&mut self) {
        // regions 1 to 4, and tex.ch's control sequences above `eqtb_size`
        let (ho, hn) = (self.o.sc("hash_high"), self.n.sc("hash_high"));
        self.eq("hash_high", ho, hn);
        let high = EQTB_SIZE + 1..=EQTB_SIZE + ho.max(hn);
        for p in (1..INT_BASE).chain(high) {
            let (x, y) = (self.o.eqtb(p), self.n.eqtb(p));
            if x == y {
                let t = b0(x);
                let pointer = matches!(t, CALL..=LONG_OUTER_CALL | GLUE_REF | SHAPE_REF | BOX_REF)
                    || (matches!(t, TOKS_REGISTER | REGISTER)
                        && !(MEM_BOT..=LO_MEM_STAT_MAX).contains(&rh(x)));
                if !pointer {
                    continue;
                }
            }
            self.eqtb_word(x, y);
            if self.err.is_some() {
                return;
            }
        }
        let _ = (GLUE_BASE, LOCAL_BASE);
    }

    /// The save stack, group by group as `show_save_groups` reads it:
    /// above each group's boundary its restore entries, below it e-TeX's
    /// saved line and the group's own saved words.
    fn save_stack(&mut self) {
        let (so, sn) = (self.o.sc("save_ptr"), self.n.sc("save_ptr"));
        self.eq("save_ptr", so, sn);
        let (bo, bn) = (self.o.sc("cur_boundary"), self.n.sc("cur_boundary"));
        self.eq("cur_boundary", bo, bn);
        let (go, gn) = (self.o.sc("cur_group"), self.n.sc("cur_group"));
        self.eq("cur_group", go, gn);
        if self.err.is_some() {
            return;
        }
        let etex = self.o.sc("eTeX_mode") == 1;
        let mut top = so; // the entries of the current group are below this
        let mut bnd = bo;
        let mut grp = go;
        let mut p = self.o.sc("nest_ptr");
        let mut a: i32 = 1;
        let mut steps = 0;
        loop {
            steps += 1;
            if steps > 100_000 {
                fail!(self, "save stack groups do not end");
            }
            // restore entries: (bnd, top)
            self.restore_entries(bnd + 1, top);
            if self.err.is_some() || grp == BOTTOM_LEVEL {
                return;
            }
            // the group's saved words below its boundary
            let mut m;
            loop {
                m = if p >= 0 && p <= self.o.sc("nest_ptr") {
                    if p == self.o.sc("nest_ptr") {
                        let r: list_state_record = rec_from(&self.o.sc_bytes("cur_list"));
                        r.mode_field
                    } else {
                        self.o.nest(p).mode_field
                    }
                } else {
                    VMODE
                };
                if p > 0 {
                    p -= 1;
                } else {
                    m = VMODE;
                }
                if m != HMODE {
                    break;
                }
            }
            let extras = match grp {
                SIMPLE_GROUP | SEMI_SIMPLE_GROUP => {
                    p += 1;
                    0
                }
                HBOX_GROUP | ADJUSTED_HBOX_GROUP | VBOX_GROUP | VTOP_GROUP => 3,
                ALIGN_GROUP => {
                    if a == 0 {
                        a = 1;
                        2
                    } else {
                        if p >= a {
                            p -= a;
                        }
                        a = 0;
                        0
                    }
                }
                NO_ALIGN_GROUP => {
                    p += 1;
                    a = -1;
                    0
                }
                OUTPUT_GROUP | MATH_LEFT_GROUP => 0,
                MATH_GROUP => {
                    p += 1;
                    1
                }
                DISC_GROUP | MATH_CHOICE_GROUP | INSERT_GROUP => {
                    p += 1;
                    1
                }
                VCENTER_GROUP => 2,
                MATH_SHIFT_GROUP => {
                    let inner_mm =
                        p >= 0 && p < self.o.sc("nest_ptr") && self.o.nest(p).mode_field == MMODE;
                    if m != MMODE && inner_mm {
                        1
                    } else {
                        0
                    }
                }
                _ => fail!(self, "unknown group code {grp}"),
            };
            let base = bnd - if etex { 1 } else { 0 };
            if etex {
                let (x, y) = (self.o.save(bnd - 1), self.n.save(bnd - 1));
                self.eq("saved line", int(x), int(y));
            }
            for k in 1..=extras {
                let (x, y) = (self.o.save(base - k), self.n.save(base - k));
                if grp == MATH_GROUP {
                    // a pointer to a field of a noad
                    self.deferred_fields.push((int(x), int(y)));
                } else {
                    self.eq("saved group word", int(x), int(y));
                }
            }
            let (bx, by) = (self.o.save(bnd), self.n.save(bnd));
            if b0(bx) != LEVEL_BOUNDARY || bx != by {
                fail!(
                    self,
                    "save stack boundary at {bnd} differs or is no boundary"
                );
            }
            top = base - extras;
            grp = b1(bx);
            bnd = rh(bx);
        }
    }

    /// Typed entries in `[lo, hi)` (§268): read from the top.
    fn restore_entries(&mut self, lo: i32, hi: i32) {
        let mut t = hi - 1;
        while t >= lo {
            let (x, y) = (self.o.save(t), self.n.save(t));
            let ty = b0(x);
            self.eq("save type", ty, b0(y));
            self.eq("save level", b1(x), b1(y));
            match ty {
                RESTORE_OLD_VALUE => {
                    self.eq("saved position", rh(x), rh(y));
                    let pos = rh(x);
                    let (sx, sy) = (self.o.save(t - 1), self.n.save(t - 1));
                    if !(INT_BASE..=EQTB_SIZE).contains(&pos) {
                        self.eqtb_word(sx, sy);
                    } else {
                        self.eq("saved value", int(sx), int(sy));
                    }
                    t -= 2;
                }
                RESTORE_ZERO | INSERT_TOKEN => {
                    self.eq("save index", rh(x), rh(y));
                    t -= 1;
                }
                RESTORE_SA => {
                    self.ptr(K::SaChain, rh(x), rh(y));
                    t -= 1;
                }
                _ => fail!(self, "save entry {t} of type {ty}"),
            }
            if self.err.is_some() {
                return;
            }
        }
    }

    fn nest_record(&mut self, x: &list_state_record, y: &list_state_record) {
        self.eq("mode", x.mode_field, y.mode_field);
        self.eq("prev_graf", x.pg_field, y.pg_field);
        self.eq("mode_line", x.ml_field, y.ml_field);
        self.ptr(K::List, x.head_field, y.head_field);
        self.later("tail", x.tail_field, y.tail_field);
        let m = x.mode_field.abs();
        let (ax, ay) = (x.aux_field.to_bits(), y.aux_field.to_bits());
        match m {
            HMODE => {
                // LR_save: an LR stack
                self.ptr(K::Lr, x.eTeX_aux_field, y.eTeX_aux_field);
                self.eq("space_factor", lh(ax), lh(ay));
                self.eq("clang", rh(ax), rh(ay));
            }
            MMODE => {
                if x.mode_field == MMODE {
                    // LR_box: a display's prototype box
                    self.ptr(K::Node, x.eTeX_aux_field, y.eTeX_aux_field);
                } else {
                    // delim_ptr: into the current mlist
                    self.later("delim_ptr", x.eTeX_aux_field, y.eTeX_aux_field);
                }
                // incompleat_noad
                self.ptr(K::Node, int(ax), int(ay));
            }
            _ => {
                self.eq("eTeX_aux", x.eTeX_aux_field, y.eTeX_aux_field);
                self.eq("prev_depth", int(ax), int(ay));
            }
        }
    }

    fn nest(&mut self) {
        let (po, pn) = (self.o.sc("nest_ptr"), self.n.sc("nest_ptr"));
        self.eq("nest_ptr", po, pn);
        if self.err.is_some() {
            return;
        }
        for k in 0..po {
            let (x, y) = (self.o.nest(k), self.n.nest(k));
            self.nest_record(&x, &y);
        }
        let x: list_state_record = rec_from(&self.o.sc_bytes("cur_list"));
        let y: list_state_record = rec_from(&self.n.sc_bytes("cur_list"));
        self.nest_record(&x, &y);
    }

    fn input_record(&mut self, x: &in_state_record, y: &in_state_record) {
        self.eq("input state", x.state_field, y.state_field);
        self.eq("input index", x.index_field, y.index_field);
        self.eq("input name", x.name_field, y.name_field);
        if x.state_field == TOKEN_LIST {
            let t = x.index_field;
            if t >= MACRO {
                self.ptr(K::Tok, x.start_field, y.start_field);
            } else if t == U_TEMPLATE || t == V_TEMPLATE {
                // a template of the preamble
                self.later("template", x.start_field, y.start_field);
            } else {
                self.ptr(K::Chain, x.start_field, y.start_field);
            }
            self.later("loc", x.loc_field, y.loc_field);
            if t == MACRO {
                self.eq("param_start", x.limit_field, y.limit_field);
            } else {
                self.eq("limit", x.limit_field, y.limit_field);
            }
            let _ = PARAMETER;
        } else {
            self.eq("start", x.start_field, y.start_field);
            self.eq("loc", x.loc_field, y.loc_field);
            self.eq("limit", x.limit_field, y.limit_field);
        }
    }

    fn input(&mut self) {
        let (po, pn) = (self.o.sc("input_ptr"), self.n.sc("input_ptr"));
        self.eq("input_ptr", po, pn);
        if self.err.is_some() {
            return;
        }
        for k in 0..po {
            let (x, y) = (self.o.input(k), self.n.input(k));
            self.input_record(&x, &y);
        }
        let x: in_state_record = rec_from(&self.o.sc_bytes("cur_input"));
        let y: in_state_record = rec_from(&self.n.sc_bytes("cur_input"));
        self.input_record(&x, &y);
        let (qo, qn) = (self.o.sc("param_ptr"), self.n.sc("param_ptr"));
        self.eq("param_ptr", qo, qn);
        for k in 0..qo.max(0) as usize {
            let (a, b) = (
                self.o.i32_at(self.o.l.param_stack, k),
                self.n.i32_at(self.n.l.param_stack, k),
            );
            self.ptr(K::Chain, a, b);
        }
    }

    fn arrays(&mut self) {
        let l = self.o.l;
        // cur_mark[0..4] (the Rust array starts at top_mark_code = 0)
        for k in 0..5 {
            let (a, b) = (self.o.i32_at(l.cur_mark, k), self.n.i32_at(l.cur_mark, k));
            self.ptr(K::Tok, a, b);
        }
        // disc_ptr[copy_code..vsplit_code]: the Rust array may start at 0
        for k in 0..4 {
            let (a, b) = (self.o.i32_at(l.disc_ptr, k), self.n.i32_at(l.disc_ptr, k));
            self.ptr(K::List, a, b);
        }
        // sa_root[int_val..mark_val]
        for k in 0..=MARK_VAL as usize {
            let (a, b) = (self.o.i32_at(l.sa_root, k), self.n.i32_at(l.sa_root, k));
            self.ptr(K::SaIdx(1), a, b);
        }
        // if_stack[0..in_open]: the conditional at each file level
        let io = self.o.sc("in_open");
        for k in 0..=io.max(0) as usize {
            let (a, b) = (self.o.i32_at(l.if_stack, k), self.n.i32_at(l.if_stack, k));
            self.later("if_stack", a, b);
        }
        // font_glue[font_base..font_ptr]
        let fp = self.o.sc("font_ptr");
        self.eq("font_ptr", fp, self.n.sc("font_ptr"));
        for k in 0..=fp.max(0) as usize {
            let (a, b) = (self.o.i32_at(l.font_glue, k), self.n.i32_at(l.font_glue, k));
            self.ptr(K::Glue, a, b);
        }
        // hyph_list[0..hyph_size]
        let hs = self
            .o
            .l
            .scalars
            .get("hyph_size")
            .map(|_| self.o.sc("hyph_size"))
            .unwrap_or(0);
        let _ = hs;
        let n_hyph = self.hyph_len;
        for k in 0..n_hyph {
            // a list only where the slot holds a word (hyph_word 0: empty,
            // its list pointer stale)
            let (wo, wn) = (self.o.i32_at(l.hyph_word, k), self.n.i32_at(l.hyph_word, k));
            self.eq("hyph_word", wo, wn);
            if wo == 0 {
                continue;
            }
            let (a, b) = (self.o.i32_at(l.hyph_list, k), self.n.i32_at(l.hyph_list, k));
            self.ptr(K::Hyph, a, b);
        }
        // pdfTeX's link stack
        let lp = self.o.sc("pdf_link_stack_ptr");
        self.eq("pdf_link_stack_ptr", lp, self.n.sc("pdf_link_stack_ptr"));
        let size = std::mem::size_of::<crate::generated::types::pdf_link_stack_record>();
        for k in 1..=lp.max(0) as usize {
            let at = |s: &St, f: usize| s.i32_at(l.pdf_link_stack + k * size, f);
            let _ = at;
            let rx: crate::generated::types::pdf_link_stack_record =
                rec_from(&self.o.bytes_at(l.pdf_link_stack + k * size, size));
            let ry: crate::generated::types::pdf_link_stack_record =
                rec_from(&self.n.bytes_at(l.pdf_link_stack + k * size, size));
            self.eq("link nesting level", rx.nesting_level, ry.nesting_level);
            self.ptr(K::Node, rx.link_node, ry.link_node);
            self.later("ref_link_node", rx.ref_link_node, ry.ref_link_node);
        }
        self.objects();
    }

    /// pdfTeX's objects that point into mem: raw objects' data, forms'
    /// boxes and token lists, images' attributes, annotations, links,
    /// destinations, threads and beads (`obj_aux`, `pdf_mem`).
    fn objects(&mut self) {
        let l = self.o.l;
        let op = self.o.sc("obj_ptr");
        self.eq("obj_ptr", op, self.n.sc("obj_ptr"));
        let sp = self.o.sc("sys_obj_ptr");
        self.eq("sys_obj_ptr", sp, self.n.sc("sys_obj_ptr"));
        if self.err.is_some() {
            return;
        }
        // each type's list: head_tab[t] (the Rust array starts at 1)
        for t in 1..=crate::generated::consts::pdf_objtype_max {
            let (mut ko, mut kn) = (
                self.o.i32_at(l.head_tab, t as usize - 1),
                self.n.i32_at(l.head_tab, t as usize - 1),
            );
            let mut steps = 0;
            while ko != 0 || kn != 0 {
                steps += 1;
                if ko != kn || steps > 1 << 24 {
                    fail!(self, "object lists of type {t} differ");
                }
                let (eo, en) = (self.o.obj(ko), self.n.obj(kn));
                self.eq("obj_info", eo.int0, en.int0);
                self.eq("obj_os_idx", eo.int3, en.int3);
                self.object(t, &eo, &en);
                if self.err.is_some() {
                    return;
                }
                ko = eo.int1;
                kn = en.int1;
            }
        }
    }

    fn pdf_mem(&self, s: &St, k: i32) -> i32 {
        s.i32_at(s.l.pdf_mem, k as usize)
    }

    fn object(&mut self, t: i32, eo: &obj_entry, en: &obj_entry) {
        const OUTLINE: i32 = 4;
        const DEST: i32 = 5;
        const STRUCT_DEST: i32 = 6;
        const OBJ: i32 = 7;
        const XFORM: i32 = 8;
        const XIMAGE: i32 = 9;
        const THREAD: i32 = 10;
        let (ao, an) = (eo.int4, en.int4);
        match t {
            DEST | STRUCT_DEST => {
                // obj_dest_ptr: null while the destination is undefined;
                // else its whatsit, in a list until the page with it is
                // written, after which only its being non-null is read
                // (pdf_fix_dest)
                self.eq(
                    "obj_dest_ptr null",
                    (ao == NULL) as i32,
                    (an == NULL) as i32,
                );
                if eo.int2 < 0 {
                    self.later("obj_dest_ptr", ao, an);
                }
            }
            // obj_thread_first: an index into pdf_mem
            THREAD => self.eq("obj_thread_first", ao, an),
            OBJ | XFORM | XIMAGE | OUTLINE => {
                // obj_aux: a pdf_mem block (its index is data)
                self.eq("obj_data_ptr", ao, an);
                if ao <= 0 || eo.int2 > -1 {
                    return;
                }
                let po = |s: &Self, k: i32| (s.pdf_mem(&s.o, ao + k), s.pdf_mem(&s.n, an + k));
                match t {
                    OBJ => {
                        // data (a token list or string), is_stream,
                        // stream_attr, is_file
                        let (x, y) = po(self, 0);
                        let (sx, sy) = po(self, 1);
                        self.eq("obj is_stream", sx, sy);
                        let (fx, fy) = po(self, 3);
                        self.eq("obj is_file", fx, fy);
                        self.ptr(K::Chain, x, y);
                        let (x, y) = po(self, 2);
                        self.ptr(K::Chain, x, y);
                    }
                    XFORM => {
                        for k in 0..3 {
                            let (x, y) = po(self, k);
                            self.eq("xform dimension", x, y);
                        }
                        let (x, y) = po(self, 3);
                        self.ptr(K::Node, x, y);
                        for k in 4..6 {
                            let (x, y) = po(self, k);
                            self.ptr(K::Chain, x, y);
                        }
                    }
                    XIMAGE => {
                        for k in 0..3 {
                            let (x, y) = po(self, k);
                            self.eq("ximage dimension", x, y);
                        }
                        let (x, y) = po(self, 3);
                        self.ptr(K::Chain, x, y);
                        let (x, y) = po(self, 4);
                        self.eq("ximage data", x, y);
                    }
                    _ => {
                        // outlines: the title is an object number, the
                        // links are object numbers, the attributes a token
                        // list (or 0)
                        for k in 0..8 {
                            let (x, y) = po(self, k);
                            if k == 7 {
                                self.ptr(K::Chain, x, y);
                            } else {
                                self.eq("outline field", x, y);
                            }
                        }
                    }
                }
            }
            _ => self.eq("obj_aux", ao, an),
        }
    }
}

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

impl<'a> Iso<'a> {
    fn finish(&mut self) {
        self.run();
        if self.err.is_some() {
            return;
        }
        let deferred = std::mem::take(&mut self.deferred);
        for (what, a, b) in deferred {
            if a == NULL && b == NULL {
                continue;
            }
            match self.partner(a) {
                Some(p) if p == b => {}
                Some(_) => fail!(self, "{what}: {a} is paired with another node than {b}"),
                None => {
                    // not reached by the walk: only the same address in both
                    // states is safe, and only for a static head
                    if !(a == b && (a >= HI_MEM_STAT_MIN || a <= LO_MEM_STAT_MAX)) {
                        fail!(self, "{what}: {a}/{b} points at nothing the walk reached");
                    }
                }
            }
        }
        let fields = std::mem::take(&mut self.deferred_fields);
        for (a, b) in fields {
            let ok = (1..=5).any(|k| self.partner(a - k) == Some(b - k));
            if !ok {
                fail!(self, "a saved math field {a}/{b} is not in paired noads");
            }
        }
    }

    /// Compare O (the old run's checkpoint, through `d`) with the live state.
    /// `Ok(nodes compared)` or `Err(why not the same)`. `bad_mem` are the
    /// differing mem words left by the byte comparison: each must lie in a
    /// cell the walk compared in both states, or free in both.
    #[allow(clippy::too_many_arguments)]
    pub fn check(
        g: &Globals,
        d: &ChunkDiff,
        slots: &[ScalarSlot],
        free_o: Option<&[u64]>,
        free_n: Option<&[u64]>,
        bad_mem: &[usize],
        hyph_len: usize,
        dest_dims_dead: bool,
        stop: &'a mut dyn FnMut() -> bool,
    ) -> Result<usize, String> {
        let l = Layout::new(g, slots);
        let live = Live {
            bytes: g.arena.bytes(),
        };
        let old = Old {
            table: d.table(&g.arena),
            d,
            g,
        };
        let o = St {
            sp: &old,
            l: &l,
            hi_mem_min: 0,
        };
        let n = St {
            sp: &live,
            l: &l,
            hi_mem_min: 0,
        };
        let (ho, hn) = (o.sc("hi_mem_min"), n.sc("hi_mem_min"));
        let o = St {
            hi_mem_min: ho,
            ..o
        };
        let n = St {
            hi_mem_min: hn,
            ..n
        };
        let mut w = Iso::new(o, n);
        w.hyph_len = hyph_len;
        w.dest_dims_dead = dest_dims_dead;
        w.stop = Some(stop);
        w.roots();
        w.finish();
        if let Some(e) = w.err {
            return Err(e);
        }
        for &p in bad_mem {
            let p = p as i32;
            let ok_o = bit(&w.cov_o, p) || free_o.is_some_and(|f| bit(f, p));
            let ok_n = bit(&w.cov_n, p) || free_n.is_some_and(|f| bit(f, p));
            if !(ok_o && ok_n) {
                return Err(format!(
                    "mem[{p}] differs outside what the walk compared ({})",
                    if ok_o { "new" } else { "old" }
                ));
            }
        }
        Ok(w.nodes)
    }

    /// Walk the live state alone and sort the allocated cells no root
    /// reaches (`free` = the free cells) into leaked ones -- nothing points
    /// at the node they belong to but other unreached cells or data (hash
    /// texts, token values, font and character codes) -- and missed ones,
    /// which something the walk did not follow points at: a missing root
    /// or node field. Returns (nodes walked, missed cells (at most `max`),
    /// missed count, leaked count).
    pub fn classify_unreached(
        g: &Globals,
        slots: &[ScalarSlot],
        free: &[u64],
        hyph_len: usize,
        max: usize,
    ) -> Result<(usize, Vec<i32>, usize, usize), String> {
        let l = Layout::new(g, slots);
        let live = Live {
            bytes: g.arena.bytes(),
        };
        let o = St {
            sp: &live,
            l: &l,
            hi_mem_min: g.hi_mem_min,
        };
        let n = St {
            sp: &live,
            l: &l,
            hi_mem_min: g.hi_mem_min,
        };
        let mut w = Iso::new(o, n);
        w.hyph_len = hyph_len;
        w.roots();
        w.finish();
        if let Some(e) = w.err {
            return Err(e);
        }
        let unreached = |p: i32| !bit(&w.cov_o, p) && !bit(free, p);
        let cells: Vec<i32> = (MEM_BOT..=g.lo_mem_max)
            .chain(g.hi_mem_min..=g.mem_end)
            .filter(|&p| unreached(p))
            .collect();
        if cells.is_empty() {
            return Ok((w.nodes, vec![], 0, 0));
        }
        // Every 32-bit value in the space that is an unreached cell, and
        // where it is: which unreached cells something reaches.
        let mut target = vec![0u64; (MEM_TOP as usize + 64) / 64 + 1];
        for &p in &cells {
            set(&mut target, p);
        }
        let bytes = g.arena.bytes();
        let mut pointed_from_outside = vec![0u64; target.len()];
        let data_region = |name: &str| {
            matches!(
                name,
                "xord"
                    | "xchr"
                    | "str_pool"
                    | "str_start"
                    | "font_info"
                    | "trie_trl"
                    | "trie_tro"
                    | "trie_trc"
                    | "trie"
                    | "hyf_next"
                    | "hyf_num"
                    | "hyf_distance"
                    | "buffer"
                    | "pdf_op_buf"
                    | "pdf_os_buf"
                    | "hash"
                    | "prim"
                    | "trick_buf"
                    | "dvi_buf"
                    | "(scalars)"
            )
        };
        let mem_off = l.mem;
        for c in 0..g.arena.chunks() {
            if !g.arena.touched(c) {
                continue;
            }
            let base = c * CHUNK_BYTES;
            for k in (0..CHUNK_BYTES).step_by(4) {
                let off = base + k;
                let v = i32::from_le_bytes(bytes[off..off + 4].try_into().unwrap());
                if !Self::in_mem(v) || !bit(&target, v) {
                    continue;
                }
                let (r, rel) = g.arena.region_at(off);
                if r.name == "mem" {
                    let cell = ((off - mem_off) / 8) as i32;
                    let high = (off - mem_off) % 8 == 4;
                    // from another unreached cell, from a free cell, or from
                    // the info half of a one-word node (a token, a character)
                    if unreached(cell) || bit(free, cell) || (high && cell >= g.hi_mem_min) {
                        continue;
                    }
                } else if r.name == "(scalars)" {
                    // a scalar the walk holds dead (a stale pointer)
                    let name = slots
                        .iter()
                        .rev()
                        .find(|s| s.off <= rel)
                        .map_or("", |s| s.name);
                    if DEAD_SCALARS.contains(&name) {
                        continue;
                    }
                } else if data_region(r.name) {
                    continue;
                }
                set(&mut pointed_from_outside, v);
            }
        }
        // A node is referenced at its first cell; an unreached cell counts as
        // missed if the start of its run of unreached cells is referenced.
        let mut missed = vec![];
        let (mut n_missed, mut n_leaked) = (0usize, 0usize);
        let mut run_missed = false;
        for &p in &cells {
            if !(p > MEM_BOT && unreached(p - 1)) {
                run_missed = false;
            }
            if bit(&pointed_from_outside, p) {
                run_missed = true;
            }
            if run_missed {
                n_missed += 1;
                if missed.len() < max {
                    missed.push(p);
                }
            } else {
                n_leaked += 1;
            }
        }
        Ok((w.nodes, missed, n_missed, n_leaked))
    }

    /// Walk the live state alone: the allocated cells no root reaches
    /// (`free` = the free cells), at most `max` of them, with the count.
    pub fn walk_one(
        g: &Globals,
        slots: &[ScalarSlot],
        free: &[u64],
        hyph_len: usize,
        max: usize,
    ) -> Result<(usize, Vec<i32>, usize), String> {
        let l = Layout::new(g, slots);
        let live = Live {
            bytes: g.arena.bytes(),
        };
        let o = St {
            sp: &live,
            l: &l,
            hi_mem_min: g.hi_mem_min,
        };
        let n = St {
            sp: &live,
            l: &l,
            hi_mem_min: g.hi_mem_min,
        };
        let mut w = Iso::new(o, n);
        w.hyph_len = hyph_len;
        w.roots();
        w.finish();
        if let Some(e) = w.err {
            return Err(e);
        }
        let mut missed = vec![];
        let mut count = 0;
        let hi_end = g.mem_end;
        let mut ranges: Vec<(i32, i32)> = vec![];
        for p in (MEM_BOT..=g.lo_mem_max).chain(g.hi_mem_min..=hi_end) {
            if !bit(&w.cov_o, p) && !bit(free, p) {
                count += 1;
                if missed.len() < max {
                    missed.push(p);
                }
                match ranges.last_mut() {
                    Some(r) if r.1 + 1 == p => r.1 = p,
                    _ => ranges.push((p, p)),
                }
            }
        }
        if std::env::var_os("FLASHTEX_CHECKMEM_RANGES").is_some() {
            let roots: Vec<String> = (0..7)
                .map(|k| {
                    let r = g.sa_root[k];
                    format!("{k}:{r}:{}", if r > 0 { bit(&w.head_o, r) } else { false })
                })
                .collect();
            eprintln!("[checkmem]   sa_root {}", roots.join(" "));
            let lo: Vec<String> = ranges
                .iter()
                .take(80)
                .map(|&(a, b)| {
                    let x = g.mem[a as usize].to_bits();
                    format!("{a}-{b}[t{} s{} l{}]", b0(x), b1(x), rh(x))
                })
                .collect();
            eprintln!("[checkmem]   {} ranges: {}", ranges.len(), lo.join(" "));
        }
        Ok((w.nodes, missed, count))
    }
}

/// Debugging `checkmem`: where in the live space a 32-bit value equal to
/// one of `targets` appears (array and element, or scalar), at most `max`.
pub fn who_points(g: &Globals, slots: &[ScalarSlot], targets: &[i32], max: usize) -> Vec<String> {
    let bytes = g.arena.bytes();
    let mut out = vec![];
    for c in 0..g.arena.chunks() {
        if !g.arena.touched(c) {
            continue;
        }
        let base = c * CHUNK_BYTES;
        for k in (0..CHUNK_BYTES).step_by(4) {
            let off = base + k;
            let v = i32::from_le_bytes(bytes[off..off + 4].try_into().unwrap());
            if targets.contains(&v) {
                let (r, rel) = g.arena.region_at(off);
                if matches!(
                    r.name,
                    "xord"
                        | "xchr"
                        | "str_pool"
                        | "str_start"
                        | "font_info"
                        | "trie_trl"
                        | "trie_tro"
                        | "trie_trc"
                        | "buffer"
                        | "pdf_op_buf"
                        | "pdf_os_buf"
                ) {
                    continue;
                }
                let what = if r.name == "(scalars)" {
                    slots
                        .iter()
                        .rev()
                        .find(|s| s.off <= rel)
                        .map(|s| s.name.to_string())
                        .unwrap_or_default()
                } else {
                    format!(
                        "{}[{}]+{}",
                        r.name,
                        rel / r.elem.max(1),
                        rel % r.elem.max(1)
                    )
                };
                out.push(format!("{v}@{what}"));
                if out.len() >= max {
                    return out;
                }
            }
        }
    }
    out
}
