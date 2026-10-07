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
    }
}

struct Ser<'a> {
    g: &'a Globals,
    holes: &'a [i32],
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

    fn sub(&mut self, p: i32) -> Result<Ref, &'static str> {
        if p == 0 {
            Ok(Ref::Null)
        } else {
            Ok(Ref::List(self.list(p, 0)?))
        }
    }

    fn big(
        &mut self,
        p: i32,
        size: i32,
        synctex: bool,
        fields: Vec<(i32, bool, Ref)>,
    ) -> Node {
        let copy = if synctex { size - SYNCTEX_FIELDS } else { size };
        let words = (0..copy).map(|w| self.g.mem[(p + w) as usize].0).collect();
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
                Ok(self.big(p, k::medium_node_size, true, vec![(1, true, s), (1, false, l)]))
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
                Ok(self.big(p, k::small_node_size, false, vec![(1, true, a), (1, false, b)]))
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
                s if s == k::pdf_restore_node => {
                    Ok(self.big(p, k::pdf_restore_node_size, false, vec![]))
                }
                _ => Err("WhatsitKind"),
            },
            _ => Err("NodeKind"),
        }
    }
}

impl Globals {
    /// The nodes from `first` up to (not including) `stop`, by value, with
    /// `holes` (box nodes the call moved out of foreign registers) as holes;
    /// each must occur exactly once.
    pub(crate) fn frag_take(&self, first: i32, stop: i32, holes: &[i32]) -> Result<Frag, &'static str> {
        let mut s = Ser {
            g: self,
            holes,
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
                return Err("SharedGlue");
            }
        }
        for (&l, &i) in &s.tok_ix {
            let refs = self.mem[l as usize].hh().lh() as i64 + 1;
            if refs != s.frag.toks[i].1 as i64 {
                return Err("SharedList");
            }
        }
        let mut f = s.frag;
        f.nodes = nodes;
        Ok(f)
    }

    /// Rebuild `f` with `holes[i]` (box nodes now) for its holes. Returns the
    /// first and the last node of the top-level list (0, 0 if empty).
    pub(crate) fn frag_make(&mut self, f: &Frag, holes: &[i32]) -> (i32, i32) {
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
        self.frag_list(&f.nodes, holes, &specs, &toks)
    }

    fn frag_list(&mut self, v: &[Node], holes: &[i32], specs: &[i32], toks: &[i32]) -> (i32, i32) {
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
                            Ref::List(l) => self.frag_list(l, holes, specs, toks).0,
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
                            Ref::Spec(Spec::Own(i)) => specs[*i],
                            Ref::Toks(i) => toks[*i],
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
