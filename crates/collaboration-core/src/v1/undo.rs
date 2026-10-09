//! Local undo in CRDT ids (contract §2.7), the oracle's version of the
//! Swift `TextUndoManager`, so that the oracle's simulations and recorded
//! fixtures contain undo and redo operations too. A step records the id
//! spans its user inserted and deleted; undoing deletes the still-visible
//! own insertions (following the copies an earlier undo or redo restored
//! them as) and re-inserts each deleted piece after its tombstone. Its
//! inverse is the redo step. Never another replica's scalars.

use std::collections::{HashMap, HashSet};

use super::text::{TextDoc, TextOp};
use super::Id;

#[derive(Debug, Clone, Copy)]
struct Span {
    first: Id,
    count: u64,
}

#[derive(Debug, Clone, Default)]
struct Step {
    inserted: Vec<Span>,
    deleted: Vec<Span>,
}

impl Step {
    fn add(&mut self, ops: &[TextOp]) {
        for op in ops {
            match op {
                TextOp::Insert { id, .. } => self.inserted.push(Span {
                    first: *id,
                    count: op.len(),
                }),
                TextOp::Delete { target, len, .. } => self.deleted.push(Span {
                    first: *target,
                    count: *len,
                }),
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.inserted.is_empty() && self.deleted.is_empty()
    }

    fn spans(&self) -> impl Iterator<Item = &Span> {
        self.inserted.iter().chain(self.deleted.iter())
    }
}

#[derive(Debug, Clone)]
pub struct UndoManager {
    undo: Vec<Step>,
    redo: Vec<Step>,
    copies: HashMap<Id, Id>,
    pub limit: usize,
}

impl Default for UndoManager {
    fn default() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            copies: HashMap::new(),
            limit: 256,
        }
    }
}

impl UndoManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one local edit's operations as one step; clears redo.
    pub fn record(&mut self, ops: &[TextOp]) {
        let mut s = Step::default();
        s.add(ops);
        if s.is_empty() {
            return;
        }
        self.undo.push(s);
        let trimmed = self.undo.len() > self.limit;
        if trimmed {
            let extra = self.undo.len() - self.limit;
            self.undo.drain(..extra);
        }
        let cleared = !self.redo.is_empty();
        self.redo.clear();
        if trimmed || cleared {
            self.prune();
        }
    }

    pub fn undo(&mut self, doc: &mut TextDoc) -> Vec<TextOp> {
        let Some(s) = self.undo.pop() else {
            return Vec::new();
        };
        let (ops, inverse) = self.revert(doc, &s);
        if !inverse.is_empty() {
            self.redo.push(inverse);
        }
        ops
    }

    pub fn redo(&mut self, doc: &mut TextDoc) -> Vec<TextOp> {
        let Some(s) = self.redo.pop() else {
            return Vec::new();
        };
        let (ops, inverse) = self.revert(doc, &s);
        if !inverse.is_empty() {
            self.undo.push(inverse);
        }
        ops
    }

    /// Copies recorded (tests check pruning keeps this bounded).
    pub fn copies_len(&self) -> usize {
        self.copies.len()
    }

    fn subtract(span: Span, others: &[Span]) -> Vec<Span> {
        let mut pieces = vec![span];
        for o in others
            .iter()
            .filter(|o| o.first.replica == span.first.replica)
        {
            let (oa, ob) = (o.first.counter, o.first.counter + o.count);
            pieces = pieces
                .into_iter()
                .flat_map(|p| {
                    let (pa, pb) = (p.first.counter, p.first.counter + p.count);
                    if ob <= pa || pb <= oa {
                        return vec![p];
                    }
                    let mut out = Vec::new();
                    if pa < oa {
                        out.push(Span {
                            first: p.first,
                            count: oa - pa,
                        });
                    }
                    if ob < pb {
                        out.push(Span {
                            first: Id::new(p.first.replica, ob),
                            count: pb - ob,
                        });
                    }
                    out
                })
                .collect();
        }
        pieces
    }

    fn follow(&self, doc: &TextDoc, span: Span) -> Vec<Span> {
        let mut out: Vec<Span> = Vec::new();
        for k in 0..span.count {
            let mut u = span.first.offset(k);
            let mut hops = 0;
            while doc.is_deleted(u) == Some(true) && hops < 10_000 {
                match self.copies.get(&u) {
                    Some(c) => u = *c,
                    None => break,
                }
                hops += 1;
            }
            match out.last_mut() {
                Some(l)
                    if l.first.replica == u.replica && l.first.counter + l.count == u.counter =>
                {
                    l.count += 1
                }
                _ => out.push(Span { first: u, count: 1 }),
            }
        }
        out
    }

    fn revert(&mut self, doc: &mut TextDoc, s: &Step) -> (Vec<TextOp>, Step) {
        let mut ops = Vec::new();
        for span in s.deleted.iter().rev() {
            for piece in Self::subtract(*span, &s.inserted) {
                for (orig, op) in doc.reinsert_deleted(piece.first, piece.count) {
                    for k in 0..op.len() {
                        self.copies.insert(orig.offset(k), op.id().offset(k));
                    }
                    ops.push(op);
                }
            }
        }
        for span in s.inserted.iter().rev() {
            for live in self.follow(doc, *span) {
                ops.extend(doc.delete_ids(live.first, live.count));
            }
        }
        let mut inverse = Step::default();
        inverse.add(&ops);
        (ops, inverse)
    }

    /// Keep only the copies reachable from a span still on either stack.
    fn prune(&mut self) {
        let mut keep: HashSet<Id> = HashSet::new();
        for s in self.undo.iter().chain(self.redo.iter()) {
            for span in s.spans() {
                for k in 0..span.count {
                    let mut u = span.first.offset(k);
                    while let Some(c) = self.copies.get(&u) {
                        if !keep.insert(u) {
                            break;
                        }
                        u = *c;
                    }
                }
            }
        }
        self.copies.retain(|k, _| keep.contains(k));
    }
}
