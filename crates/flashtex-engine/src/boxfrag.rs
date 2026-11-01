//! BOX-MEMO's node fragments (docs/design/engine-v2/BOX-MEMO.md §10.3): the
//! nodes a draw call appended to the current list, kept by value outside the
//! word space, and rebuilt on a replay.
//!
//! The node formats are pdftex.web's, as `copy_node_list` copies them
//! (tex.web §204-§206, pdfTeX's "Make a partial copy of the whatsit node"):
//! every word of a node is kept as it is, and the pointer fields become
//! references: a sub-list (a box's list, leaders, a ligature's characters, a
//! discretionary's texts) by value, a glue specification by value or as one
//! of the static ones or as the one an `eqtb` entry holds, a token list (a
//! whatsit's data) by its tokens, and the *hole*: a box node the call moved
//! out of a register it did not make (`\box\@tempboxa`), which a replay
//! takes from the register as it is then.
//!
//! What a node's words do not hold is made as the normal path made it:
//! `get_node` and `get_avail` write SyncTeX's tag and line and the display
//! list's source position for the place TeX reads at, which is the same for
//! every node a call makes (a recorded call reads no file), so the last two
//! words of a box, rule, glue, kern, math or penalty node are not copied.
//!
//! Sharing is kept: a token list or glue specification the fragment uses
//! more than once is rebuilt once and referenced as often; one also used
//! from outside the fragment (other than a static specification or one an
//! `eqtb` glue entry holds) makes the fragment unrecordable.

use crate::generated::consts as k;
use crate::generated::Globals;
use std::collections::HashMap;

const SYNCTEX_FIELDS: i32 = 2;

/// A glue specification the fragment points at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Spec {
    /// `zero_glue` & co. (below `lo_mem_stat_max`)
    Static(i32),
    /// the one `eqtb[p]` holds when the fragment is rebuilt
    Eqtb(i32),
    /// the one e-TeX's sparse register (type, number) holds then
    Sparse(i32, i32),
    /// font `f`'s interword glue (`font_glue[f]`), which the call's key
    /// holds as it was (`boxmemo`'s font glues)
    Font(i32),
    /// one of the fragment's own, by index into `Frag::specs`
    Own(usize),
}

/// What a pointer field of a node refers to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ref {
    Null,
    List(Vec<Node>),
    Spec(Spec),
    /// one of the fragment's token lists, by index into `Frag::toks`
    Toks(usize),
    /// one of the fragment's link actions, by index into `Frag::actions`
    Action(usize),
    /// the number of the `i`-th object the call made (`pdf_new_objnum` for
    /// a `\pdfstartlink`): the replay's own `i`-th new object, written
    /// into the field's word as an integer
    NewObj(usize),
}

/// A link's action structure (pdftex.web's `scan_action`): its four words
/// (the pointer fields set again) and its token-list fields, which
/// `delete_action_ref` names as the structure's own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Action {
    pub words: [u64; 4],
    pub fields: Vec<Field>,
}

/// A pointer field: the word, which half (`true`: the left half, `info`),
/// and what it refers to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub word: i32,
    pub left: bool,
    pub to: Ref,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    /// A character node: its `info` (font and character).
    Char(i32),
    /// A node of `size` words: words `0..copy` as they were (the link and
    /// the pointer fields are set again), then its pointer fields.
    Big {
        size: i32,
        copy: i32,
        words: Vec<u64>,
        fields: Vec<Field>,
    },
    /// The box moved out of the call's `hole`-th foreign register.
    Hole(usize),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Frag {
    pub nodes: Vec<Node>,
    /// glue specifications: width, stretch, shrink, and the word with the
    /// orders; how many references the fragment makes to each
    pub specs: Vec<([u64; 4], u32)>,
    /// token lists: their tokens; how many references
    pub toks: Vec<(Vec<i32>, u32)>,
    /// link actions; how many references
    pub actions: Vec<(Action, u32)>,
    /// the fonts whose interword glue it uses (`Spec::Font`)
    pub fonts: Vec<i32>,
}

impl Frag {
    /// Rough size in bytes (the store's budget).
    pub fn bytes(&self) -> usize {
        fn n(v: &[Node]) -> usize {
            v.iter()
                .map(|x| match x {
                    Node::Big { words, fields, .. } => {
                        48 + 8 * words.len()
                            + fields
                                .iter()
                                .map(|f| match &f.to {
                                    Ref::List(l) => 32 + n(l),
                                    _ => 32,
                                })
                                .sum::<usize>()
                    }
                    _ => 16,
                })
                .sum()
        }
        n(&self.nodes)
            + 40 * self.specs.len()
            + self.toks.iter().map(|t| 24 + 4 * t.0.len()).sum::<usize>()
            + 96 * self.actions.len()
    }
}

/// The bits of word `w` of a node of type `t` and subtype `st` that the
/// program reads: a word that holds one `scaled` or `integer` (`.sc`,
/// `.int`, the low half) leaves its other half as `get_node` found it, so
/// two runs that made the same node may differ there (tex.web §135 ff.,
/// pdftex.web's whatsit layouts). The verifier compares only these bits.
pub fn meaningful(t: i32, st: i32, w: usize) -> u64 {
    const LOW: u64 = 0xffff_ffff;
    const HIGH: u64 = !LOW;
    if w == 0 {
        return !0;
    }
    if t == k::hlist_node || t == k::vlist_node {
        // width, depth, height, shift_amount; list and glue; glue_set
        return if w <= 4 { LOW } else { !0 };
    }
    if t == k::unset_node {
        // ... and glue_stretch in the glue_set word
        return if w <= 4 || w == 6 { LOW } else { !0 };
    }
    if t == k::rule_node {
        return if w <= 3 { LOW } else { !0 };
    }
    if t == k::kern_node || t == k::math_node || t == k::penalty_node {
        return if w == 1 { LOW } else { !0 };
    }
    if t == k::whatsit_node {
        if st == k::pdf_start_link_node {
            // width, height, depth; word 4 is not used; the object number
            return match w {
                1..=3 | 6 => LOW,
                4 => 0,
                _ => !0,
            };
        }
        if st == k::pdf_colorstack_node {
            // a setter's data is word 2's link; its info is not used
            return if w == 2 { LOW } else { !0 };
        }
        if st == k::pdf_setmatrix_node {
            return if w == 1 { LOW } else { !0 };
        }
        if st == k::pdf_save_node || st == k::pdf_restore_node || st == k::pdf_end_link_node {
            return 0;
        }
        if st == k::close_node {
            // `write_stream` (info); `write_tokens` is not set
            return if w == 1 { HIGH } else { !0 };
        }
        if st == k::pdf_refxform_node || st == k::pdf_refximage_node {
            // width, height, depth; the object number (`info`)
            return match w {
                1..=3 => LOW,
                4 => HIGH,
                _ => !0,
            };
        }
    }
    !0
}

/// Where two node lists first differ (the verifier's report): the path of
/// node indices and the two parts, deepest first. Only the bits the
/// program reads are compared ([`meaningful`]).
pub fn first_diff(a: &[Node], b: &[Node]) -> Option<String> {
    let same_words = |x: &[u64], y: &[u64]| -> bool {
        if x.len() != y.len() {
            return false;
        }
        let (t, st) = (
            ((x[0] >> 48) & 0xffff) as i32,
            ((x[0] >> 32) & 0xffff) as i32,
        );
        x.iter()
            .zip(y)
            .enumerate()
            .all(|(j, (p, q))| (p ^ q) & meaningful(t, st, j) == 0)
    };
    for (i, (x, y)) in a.iter().zip(b).enumerate() {
        if x == y {
            continue;
        }
        if let (
            Node::Big {
                words: wa,
                fields: fa,
                size: sa,
                ..
            },
            Node::Big {
                words: wb,
                fields: fb,
                size: sb,
                ..
            },
        ) = (x, y)
        {
            if sa == sb && fa.len() == fb.len() {
                for (f, g) in fa.iter().zip(fb) {
                    if let (Ref::List(l), Ref::List(m)) = (&f.to, &g.to) {
                        if let Some(d) = first_diff(l, m) {
                            return Some(format!("{i}.{}: {d}", f.word));
                        }
                    } else if f != g {
                        return Some(format!("{i}.{}: {:?} / {:?}", f.word, f.to, g.to));
                    }
                }
                if same_words(wa, wb) {
                    continue;
                }
            }
            if !same_words(wa, wb) {
                let w: Vec<String> = wa
                    .iter()
                    .zip(wb)
                    .enumerate()
                    .filter(|(_, (p, q))| p != q)
                    .map(|(j, (p, q))| format!("word {j}: {p:#x} / {q:#x}"))
                    .collect();
                return Some(format!(
                    "{i} (size {sa}, type word {:#x}): {}",
                    wa[0],
                    w.join(", ")
                ));
            }
        }
        return Some(format!("{i}: {x:?} / {y:?}").chars().take(300).collect());
    }
    (a.len() != b.len()).then(|| format!("{} nodes / {}", a.len(), b.len()))
}

/// Where two fragments first differ in what the program reads, or `None`
/// (the verifier's comparison: [`first_diff`] for the nodes; a glue
/// specification's width, stretch and shrink and an action's numbers are
/// low halves too, and an action's unused halves are left out).
pub fn frag_diff(a: &Frag, b: &Frag) -> Option<String> {
    if let Some(d) = first_diff(&a.nodes, &b.nodes) {
        return Some(format!("nodes at {d}"));
    }
    let spec_same = |x: &[u64; 4], y: &[u64; 4]| {
        x[0] == y[0] && (1..4).all(|j| (x[j] ^ y[j]) & 0xffff_ffff == 0)
    };
    if a.specs.len() != b.specs.len()
        || a.specs
            .iter()
            .zip(&b.specs)
            .any(|(x, y)| x.1 != y.1 || !spec_same(&x.0, &y.0))
    {
        return Some(format!("glue {:?} / {:?}", a.specs, b.specs));
    }
    if a.toks != b.toks {
        return Some("token lists".into());
    }
    let act_same = |x: &Action, y: &Action| {
        let user = ((x.words[0] >> 48) & 0xffff) as i32 == k::pdf_action_user;
        // (`scan_action` returns a user action before it sets the subtype,
        // `new_window` and the structure identifier; word 3's info is never
        // used, and word 2's only as a field)
        let (m0, m1, m3) = if user {
            (!(0xffffu64 << 32), !0xffff_ffffu64, 0)
        } else {
            (!0, !0, 0xffff_ffff)
        };
        x.fields == y.fields
            && (x.words[0] ^ y.words[0]) & m0 == 0
            && (x.words[1] ^ y.words[1]) & m1 == 0
            && (x.words[2] ^ y.words[2]) & 0xffff_ffff == 0
            && (x.words[3] ^ y.words[3]) & m3 == 0
    };
    if a.actions.len() != b.actions.len()
        || a.actions
            .iter()
            .zip(&b.actions)
            .any(|(x, y)| x.1 != y.1 || !act_same(&x.0, &y.0))
    {
        return Some(format!("actions {:?} / {:?}", a.actions, b.actions));
    }
    (a.fonts != b.fonts).then(|| format!("fonts {:?} / {:?}", a.fonts, b.fonts))
}

struct Ser<'a> {
    g: &'a Globals,
    holes: &'a [i32],
    /// the objects the call made, in order (`Ref::NewObj`)
    new_objs: &'a [i32],
    act_ix: HashMap<i32, usize>,
    found: Vec<u32>,
    spec_ix: HashMap<i32, usize>,
    tok_ix: HashMap<i32, usize>,
    frag: Frag,
    nodes: usize,
}

const MAX_NODES: usize = 200_000;

impl Ser<'_> {
    fn hh(&self, p: i32) -> crate::generated::types::two_halves {
        self.g.mem[p as usize].hh()
    }

    fn list(&mut self, mut p: i32, stop: i32) -> Result<Vec<Node>, &'static str> {
        let mut v = vec![];
        while p != 0 && p != stop {
            self.nodes += 1;
            if self.nodes > MAX_NODES {
                return Err("LongFragment");
            }
            v.push(self.node(p)?);
            p = self.hh(p).rh();
        }
        Ok(v)
    }

    fn spec(&mut self, q: i32) -> Result<Spec, &'static str> {
        if q <= k::lo_mem_stat_max {
            return Ok(Spec::Static(q));
        }
        if let Some(&i) = self.spec_ix.get(&q) {
            self.frag.specs[i].1 += 1;
            return Ok(Spec::Own(i));
        }
        let g = self.g;
        // held by an eqtb glue entry (a parameter or a register): shared
        for p in k::glue_base..k::local_base {
            if g.eqtb[(p - 1) as usize].hh().rh() == q {
                return Ok(Spec::Eqtb(p));
            }
        }
        // ... or a font's interword glue (`font_glue`, an interword space's)
        for f in 0..=g.font_ptr {
            if g.font_glue[f as usize] == q {
                if !self.frag.fonts.contains(&f) {
                    self.frag.fonts.push(f);
                }
                return Ok(Spec::Font(f));
            }
        }
        // ... or by one of e-TeX's sparse glue registers
        for t in [k::glue_val, k::mu_val] {
            let mut found = None;
            g.bm_sa_walk(t, &mut |n, e| {
                if found.is_none() && g.mem[(e + 1) as usize].hh().rh() == q {
                    found = Some(n);
                }
            });
            if let Some(n) = found {
                return Ok(Spec::Sparse(t, n));
            }
        }
        let w = [
            g.mem[q as usize].0,
            g.mem[(q + 1) as usize].0,
            g.mem[(q + 2) as usize].0,
            g.mem[(q + 3) as usize].0,
        ];
        self.frag.specs.push((w, 1));
        let i = self.frag.specs.len() - 1;
        self.spec_ix.insert(q, i);
        Ok(Spec::Own(i))
    }

    fn toks(&mut self, l: i32) -> Result<Ref, &'static str> {
        if l == 0 {
            return Ok(Ref::Null);
        }
        if let Some(&i) = self.tok_ix.get(&l) {
            self.frag.toks[i].1 += 1;
            return Ok(Ref::Toks(i));
        }
        let mut v = vec![];
        let mut p = self.hh(l).rh();
        while p != 0 {
            if v.len() > 1 << 20 {
                return Err("LongList");
            }
            v.push(self.hh(p).lh());
            p = self.hh(p).rh();
        }
        self.frag.toks.push((v, 1));
        let i = self.frag.toks.len() - 1;
        self.tok_ix.insert(l, i);
        Ok(Ref::Toks(i))
    }

    /// A link's action structure `a` (pdftex.web's `scan_action`), by value:
    /// its token lists as `delete_action_ref` frees them.
    fn action(&mut self, a: i32) -> Result<Ref, &'static str> {
        if a == 0 {
            return Err("LinkAction");
        }
        if let Some(&i) = self.act_ix.get(&a) {
            self.frag.actions[i].1 += 1;
            return Ok(Ref::Action(i));
        }
        let (ty, named) = (self.hh(a).b0(), self.hh(a).b1());
        let mut fields = vec![];
        if ty == k::pdf_action_user {
            fields.push((2, true, self.toks(self.hh(a + 2).lh())?));
        } else {
            fields.push((1, true, self.toks(self.hh(a + 1).lh())?));
            if ty == k::pdf_action_page {
                fields.push((2, true, self.toks(self.hh(a + 2).lh())?));
            } else if named & 1 == 1 {
                fields.push((0, false, self.toks(self.hh(a).rh())?));
            }
            if named & 2 == 2 {
                fields.push((3, false, self.toks(self.hh(a + 3).rh())?));
            }
        }
        let mut words = [0u64; 4];
        for (w, x) in words.iter_mut().enumerate() {
            *x = self.g.mem[a as usize + w].0;
        }
        // the pointer fields and the reference count are set again
        words[2] &= !0xffff_ffffu64;
        for (w, left, _) in &fields {
            let m = if *left {
                !(0xffff_ffffu64 << 32)
            } else {
                !0xffff_ffffu64
            };
            words[*w as usize] &= m;
        }
        self.frag.actions.push((
            Action {
                words,
                fields: fields
                    .into_iter()
                    .map(|(word, left, to)| Field { word, left, to })
                    .collect(),
            },
            1,
        ));
        let i = self.frag.actions.len() - 1;
        self.act_ix.insert(a, i);
        Ok(Ref::Action(i))
    }

    fn sub(&mut self, p: i32) -> Result<Ref, &'static str> {
        if p == 0 {
            Ok(Ref::Null)
        } else {
            Ok(Ref::List(self.list(p, 0)?))
        }
    }

    fn big(&mut self, p: i32, size: i32, synctex: bool, fields: Vec<(i32, bool, Ref)>) -> Node {
        let copy = if synctex { size - SYNCTEX_FIELDS } else { size };
        let mut words: Vec<u64> = (0..copy).map(|w| self.g.mem[(p + w) as usize].0).collect();
        // addresses are not kept: the link, and each pointer field
        words[0] &= !0xffff_ffffu64;
        for (w, left, _) in &fields {
            let m = if *left {
                !(0xffff_ffffu64 << 32)
            } else {
                !0xffff_ffffu64
            };
            words[*w as usize] &= m;
        }
        Node::Big {
            size,
            copy,
            words,
            fields: fields
                .into_iter()
                .map(|(word, left, to)| Field { word, left, to })
                .collect(),
        }
    }

    fn node(&mut self, p: i32) -> Result<Node, &'static str> {
        if p >= self.g.hi_mem_min {
            return Ok(Node::Char(self.hh(p).lh()));
        }
        let (t, st) = (self.hh(p).b0(), self.hh(p).b1());
        match t {
            t if t == k::hlist_node || t == k::vlist_node || t == k::unset_node => {
                if let Some(i) = self.holes.iter().position(|&h| h == p) {
                    self.found[i] += 1;
                    return Ok(Node::Hole(i));
                }
                let l = self.sub(self.hh(p + 5).rh())?;
                Ok(self.big(p, k::box_node_size, true, vec![(5, false, l)]))
            }
            t if t == k::rule_node => Ok(self.big(p, k::rule_node_size, true, vec![])),
            t if t == k::glue_node => {
                let s = Ref::Spec(self.spec(self.hh(p + 1).lh())?);
                let l = self.sub(self.hh(p + 1).rh())?;
                Ok(self.big(
                    p,
                    k::medium_node_size,
                    true,
                    vec![(1, true, s), (1, false, l)],
                ))
            }
            t if t == k::kern_node || t == k::math_node || t == k::penalty_node => {
                Ok(self.big(p, k::medium_node_size, true, vec![]))
            }
            t if t == k::ligature_node => {
                let l = self.sub(self.hh(p + 1).rh())?;
                Ok(self.big(p, k::small_node_size, false, vec![(1, false, l)]))
            }
            t if t == k::disc_node => {
                let a = self.sub(self.hh(p + 1).lh())?;
                let b = self.sub(self.hh(p + 1).rh())?;
                Ok(self.big(
                    p,
                    k::small_node_size,
                    false,
                    vec![(1, true, a), (1, false, b)],
                ))
            }
            t if t == k::whatsit_node => match st {
                s if s == k::write_node
                    || s == k::special_node
                    || s == k::latespecial_node
                    || s == k::pdf_literal_node
                    || s == k::pdf_lateliteral_node =>
                {
                    let l = self.toks(self.hh(p + 1).rh())?;
                    Ok(self.big(p, k::write_node_size, false, vec![(1, false, l)]))
                }
                s if s == k::close_node || s == k::language_node => {
                    Ok(self.big(p, k::small_node_size, false, vec![]))
                }
                s if s == k::pdf_colorstack_node => {
                    if self.hh(p + 1).lh() <= k::colorstack_data {
                        let l = self.toks(self.hh(p + 2).rh())?;
                        Ok(self.big(
                            p,
                            k::pdf_colorstack_setter_node_size,
                            false,
                            vec![(2, false, l)],
                        ))
                    } else {
                        Ok(self.big(p, k::pdf_colorstack_getter_node_size, false, vec![]))
                    }
                }
                s if s == k::pdf_setmatrix_node => {
                    let l = self.toks(self.hh(p + 1).rh())?;
                    Ok(self.big(p, k::pdf_setmatrix_node_size, false, vec![(1, false, l)]))
                }
                s if s == k::pdf_save_node => Ok(self.big(p, k::pdf_save_node_size, false, vec![])),
                // pdftex.web's "Implement \pdfstartlink": its attributes, its
                // action, and the object `pdf_new_objnum` made for it, which
                // must be one the call made
                s if s == k::pdf_start_link_node => {
                    let attr = self.toks(self.hh(p + 5).lh())?;
                    let act = self.action(self.hh(p + 5).rh())?;
                    let n = self.g.mem[(p + 6) as usize].int();
                    let i = self
                        .new_objs
                        .iter()
                        .position(|&o| o == n)
                        .ok_or("LinkObject")?;
                    Ok(self.big(
                        p,
                        k::pdf_annot_node_size,
                        false,
                        vec![(5, true, attr), (5, false, act), (6, false, Ref::NewObj(i))],
                    ))
                }
                s if s == k::pdf_end_link_node => {
                    Ok(self.big(p, k::small_node_size, false, vec![]))
                }
                // an existing form or image, by number (the call's key holds
                // the object: `boxmemo`'s object references)
                s if s == k::pdf_refxform_node => {
                    Ok(self.big(p, k::pdf_refxform_node_size, false, vec![]))
                }
                s if s == k::pdf_refximage_node => {
                    Ok(self.big(p, k::pdf_refximage_node_size, false, vec![]))
                }
                s if s == k::pdf_restore_node => {
                    Ok(self.big(p, k::pdf_restore_node_size, false, vec![]))
                }
                _ => Err("WhatsitKind"),
            },
            _ => Err("NodeKind"),
        }
    }
}

/// What a fragment's references point at once it is rebuilt.
struct Rebuild<'a> {
    holes: &'a [i32],
    specs: &'a [i32],
    toks: &'a [i32],
    acts: &'a [i32],
    objs: &'a [i32],
}

impl Globals {
    /// The nodes from `first` up to (not including) `stop`, by value, with
    /// `holes` (box nodes the call moved out of foreign registers) as holes;
    /// each must occur exactly once.
    pub(crate) fn frag_take(
        &self,
        first: i32,
        stop: i32,
        holes: &[i32],
        new_objs: &[i32],
    ) -> Result<Frag, &'static str> {
        let mut s = Ser {
            g: self,
            holes,
            new_objs,
            act_ix: HashMap::new(),
            found: vec![0; holes.len()],
            spec_ix: HashMap::new(),
            tok_ix: HashMap::new(),
            frag: Frag::default(),
            nodes: 0,
        };
        let nodes = s.list(first, stop)?;
        if s.found.iter().any(|&n| n != 1) {
            return Err("Hole");
        }
        // A list or specification used from outside the fragment too
        // (its reference count says more references than the fragment
        // makes) cannot be rebuilt as the fragment's own.
        for (&q, &i) in &s.spec_ix {
            let refs = self.mem[q as usize].hh().rh() as i64 + 1;
            if refs != s.frag.specs[i].1 as i64 {
                if std::env::var_os("FLASHTEX_BOXMEMO_DEBUG").is_some() {
                    eprintln!(
                        "boxfrag: glue specification {q} ({:?}) has {refs} references, {} from the fragment",
                        &s.frag.specs[i].0[1..],
                        s.frag.specs[i].1
                    );
                }
                return Err("SharedGlue");
            }
        }
        for (&l, &i) in &s.tok_ix {
            let refs = self.mem[l as usize].hh().lh() as i64 + 1;
            if refs != s.frag.toks[i].1 as i64 {
                return Err("SharedList");
            }
        }
        for (&a, &i) in &s.act_ix {
            // `pdf_action_refcount`: references minus one
            let refs = self.mem[(a + 2) as usize].hh().rh() as i64 + 1;
            if refs != s.frag.actions[i].1 as i64 {
                return Err("SharedAction");
            }
        }
        let mut f = s.frag;
        f.nodes = nodes;
        Ok(f)
    }

    /// Rebuild `f` with `holes[i]` (box nodes now) for its holes. Returns the
    /// first and the last node of the top-level list (0, 0 if empty).
    pub(crate) fn frag_make(&mut self, f: &Frag, holes: &[i32], objs: &[i32]) -> (i32, i32) {
        let mut specs = vec![0; f.specs.len()];
        for (i, (w, n)) in f.specs.iter().enumerate() {
            let q = self.get_node(k::glue_spec_size);
            for (j, &x) in w.iter().enumerate() {
                self.mem[q as usize + j] = crate::generated::types::memory_word(x);
            }
            // reference count: references minus one
            self.mem[q as usize].set_hh_rh(*n as i32 - 1);
            specs[i] = q;
        }
        let mut toks = vec![0; f.toks.len()];
        for (i, (v, n)) in f.toks.iter().enumerate() {
            let head = self.get_avail();
            let mut q = head;
            for &t in v {
                let r = self.get_avail();
                self.mem[r as usize].set_hh_lh(t);
                self.mem[q as usize].set_hh_rh(r);
                q = r;
            }
            self.mem[q as usize].set_hh_rh(0);
            self.mem[head as usize].set_hh_lh(*n as i32 - 1);
            toks[i] = head;
        }
        let mut acts = vec![0; f.actions.len()];
        for (i, (a, n)) in f.actions.iter().enumerate() {
            let p = self.get_node(k::pdf_action_size);
            for (j, &x) in a.words.iter().enumerate() {
                self.mem[p as usize + j] = crate::generated::types::memory_word(x);
            }
            for fl in &a.fields {
                let x = match &fl.to {
                    Ref::Toks(t) => toks[*t],
                    _ => 0,
                };
                let w = (p + fl.word) as usize;
                if fl.left {
                    self.mem[w].set_hh_lh(x);
                } else {
                    self.mem[w].set_hh_rh(x);
                }
            }
            // `pdf_action_refcount`: references minus one
            self.mem[(p + 2) as usize].set_hh_rh(*n as i32 - 1);
            acts[i] = p;
        }
        let r = Rebuild {
            holes,
            specs: &specs,
            toks: &toks,
            acts: &acts,
            objs,
        };
        self.frag_list(&f.nodes, &r)
    }

    fn frag_list(&mut self, v: &[Node], r: &Rebuild) -> (i32, i32) {
        let (holes, specs, toks) = (r.holes, r.specs, r.toks);
        let (mut first, mut last) = (0, 0);
        for n in v {
            let p = match n {
                Node::Char(info) => {
                    let p = self.get_avail();
                    self.mem[p as usize].set_hh_lh(*info);
                    p
                }
                Node::Hole(i) => holes[*i],
                Node::Big {
                    size,
                    copy,
                    words,
                    fields,
                } => {
                    let p = self.get_node(*size);
                    for (j, &w) in words.iter().enumerate().take(*copy as usize) {
                        self.mem[p as usize + j] = crate::generated::types::memory_word(w);
                    }
                    for f in fields {
                        let x = match &f.to {
                            Ref::Null => 0,
                            Ref::List(l) => self.frag_list(l, r).0,
                            Ref::Spec(Spec::Static(q)) => {
                                let r = self.mem[*q as usize].hh().rh() + 1;
                                self.mem[*q as usize].set_hh_rh(r);
                                *q
                            }
                            Ref::Spec(Spec::Eqtb(e)) => {
                                let q = self.eqtb[(*e - 1) as usize].hh().rh();
                                let r = self.mem[q as usize].hh().rh() + 1;
                                self.mem[q as usize].set_hh_rh(r);
                                q
                            }
                            Ref::Spec(Spec::Sparse(t, n)) => {
                                // (the register holds it at the end of the
                                // call, as it did at the recording's: the
                                // key and the replayed operations make the
                                // same state)
                                self.find_sa_element(*t, *n, false);
                                assert!(self.cur_ptr != 0, "boxfrag: sparse glue register gone");
                                let q = self.mem[(self.cur_ptr + 1) as usize].hh().rh();
                                let r = self.mem[q as usize].hh().rh() + 1;
                                self.mem[q as usize].set_hh_rh(r);
                                q
                            }
                            Ref::Spec(Spec::Font(f)) => {
                                let q = self.font_glue[*f as usize];
                                assert!(q != 0, "boxfrag: font glue gone");
                                let r = self.mem[q as usize].hh().rh() + 1;
                                self.mem[q as usize].set_hh_rh(r);
                                q
                            }
                            Ref::Spec(Spec::Own(i)) => specs[*i],
                            Ref::Toks(i) => toks[*i],
                            Ref::Action(i) => r.acts[*i],
                            Ref::NewObj(i) => r.objs[*i],
                        };
                        let w = (p + f.word) as usize;
                        if f.left {
                            self.mem[w].set_hh_lh(x);
                        } else {
                            self.mem[w].set_hh_rh(x);
                        }
                    }
                    p
                }
            };
            self.mem[p as usize].set_hh_rh(0);
            if first == 0 {
                first = p;
            } else {
                self.mem[last as usize].set_hh_rh(p);
            }
            last = p;
        }
        (first, last)
    }
}
