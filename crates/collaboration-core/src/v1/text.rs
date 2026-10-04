//! The text sequence CRDT, oracle edition: one entry per Unicode scalar.
//!
//! Integration is FugueMax, written exactly as Joseph Gentle's
//! `integrateYjsMod` in `reference-crdts` (which *The Art of the Fugue*
//! proves equivalent to FugueMax): an insert carries the ids of its raw
//! neighbours at creation (`origin_left`, `origin_right`, tombstones
//! included); concurrent inserts between the same neighbours are ordered by
//! the punnet square below, ties by replica id. A run `"abc"` with id `i`
//! is, unit by unit, `a(i; left, right)`, `b(i+1; i, right)`,
//! `c(i+2; i+1, right)`, and the oracle integrates it exactly so.

use std::collections::{HashMap, HashSet};

use super::{limits, Applied, CollabError, Fnv64, Id, StateVector};

/// One `collab-v1` text operation. Each consumes `len()` consecutive
/// counters of its replica.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextOp {
    /// Insert `content` (one or more scalars) between the raw neighbours
    /// `origin_left` and `origin_right` (`None`: the start, the end).
    Insert {
        id: Id,
        origin_left: Option<Id>,
        origin_right: Option<Id>,
        content: String,
    },
    /// Tombstone the `len` scalars `target, target + 1, ...` (one replica's
    /// consecutive insert ids).
    Delete { id: Id, target: Id, len: u64 },
}

impl TextOp {
    pub fn id(&self) -> Id {
        match self {
            TextOp::Insert { id, .. } | TextOp::Delete { id, .. } => *id,
        }
    }

    /// Number of units (counters) this operation consumes.
    pub fn len(&self) -> u64 {
        match self {
            TextOp::Insert { content, .. } => content.chars().count() as u64,
            TextOp::Delete { len, .. } => *len,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The same operation without its first `skip` units (`0 < skip <
    /// len`). Valid because a run is defined unit by unit.
    pub fn without_prefix(&self, skip: u64) -> TextOp {
        match self {
            TextOp::Insert {
                id,
                origin_right,
                content,
                ..
            } => TextOp::Insert {
                id: id.offset(skip),
                origin_left: Some(id.offset(skip - 1)),
                origin_right: *origin_right,
                content: content.chars().skip(skip as usize).collect(),
            },
            TextOp::Delete { id, target, len } => TextOp::Delete {
                id: id.offset(skip),
                target: target.offset(skip),
                len: len - skip,
            },
        }
    }
}

/// Which side of its anchor a [`RelativePosition`] sticks to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assoc {
    /// Just before the anchor scalar (moves with the text on its right).
    /// With no anchor: the end of the document.
    Before,
    /// Just after the anchor scalar (moves with the text on its left).
    /// With no anchor: the start of the document.
    After,
}

/// A position that survives concurrent edits: a caret, a selection end, a
/// fold, an undo anchor. It names a scalar by id, so resolving it never
/// needs edits replayed against it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelativePosition {
    pub anchor: Option<Id>,
    pub assoc: Assoc,
}

#[derive(Debug, Clone)]
struct Item {
    id: Id,
    origin_left: Option<Id>,
    origin_right: Option<Id>,
    ch: char,
    deleted: bool,
}

/// One replica of one text file.
#[derive(Debug, Clone)]
pub struct TextDoc {
    replica: u64,
    items: Vec<Item>,
    ids: HashSet<Id>,
    /// Delete unit id -> the scalar it deleted (duplicate verification).
    deletes: HashMap<Id, Id>,
    sv: StateVector,
    /// Every applied operation, in application order (a causal order).
    log: Vec<TextOp>,
    bytes: usize,
}

impl TextDoc {
    pub fn new(replica: u64) -> Self {
        Self {
            replica,
            items: Vec::new(),
            ids: HashSet::new(),
            deletes: HashMap::new(),
            sv: StateVector::default(),
            log: Vec::new(),
            bytes: 0,
        }
    }

    pub fn replica(&self) -> u64 {
        self.replica
    }

    pub fn text(&self) -> String {
        self.items
            .iter()
            .filter(|it| !it.deleted)
            .map(|it| it.ch)
            .collect()
    }

    /// Visible length in scalars.
    pub fn len(&self) -> usize {
        self.items.iter().filter(|it| !it.deleted).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Scalars stored, tombstones included.
    pub fn raw_len(&self) -> usize {
        self.items.len()
    }

    pub fn state_vector(&self) -> StateVector {
        self.sv.clone()
    }

    pub fn log(&self) -> &[TextOp] {
        &self.log
    }

    /// FNV-1a 64 over every stored scalar in document order: `replica`
    /// (u64 LE), `counter` (u64 LE), `deleted` (u8), the scalar (u32 LE).
    /// Equal digests mean equal structure, not just equal text.
    pub fn digest(&self) -> u64 {
        let mut h = Fnv64::default();
        for it in &self.items {
            h.u64(it.id.replica);
            h.u64(it.id.counter);
            h.bytes(&[u8::from(it.deleted)]);
            h.bytes(&(it.ch as u32).to_le_bytes());
        }
        h.0
    }

    /// The ids of the visible scalars, in order (test support).
    pub fn visible_ids(&self) -> Vec<Id> {
        self.items
            .iter()
            .filter(|it| !it.deleted)
            .map(|it| it.id)
            .collect()
    }

    fn index_of(&self, id: Id) -> Option<usize> {
        self.items.iter().position(|it| it.id == id)
    }

    /// Raw index of the visible scalar number `pos`, or `items.len()` when
    /// `pos` is the visible length (FugueMax's `findItemAtPos`, with
    /// `stick_end = false`: tombstones before it stay on the left).
    fn raw_index_of_visible(&self, pos: usize) -> Option<usize> {
        let mut seen = 0;
        for (i, it) in self.items.iter().enumerate() {
            if it.deleted {
                continue;
            }
            if seen == pos {
                return Some(i);
            }
            seen += 1;
        }
        (seen == pos).then_some(self.items.len())
    }

    // ------------------------------------------------------------------
    // Local edits
    // ------------------------------------------------------------------

    /// Insert `text` before visible scalar `pos`, apply it, and return the
    /// operation to broadcast. `None` if `pos` is past the end or `text`
    /// is empty.
    pub fn insert(&mut self, pos: usize, text: &str) -> Option<TextOp> {
        if text.is_empty() {
            return None;
        }
        let i = self.raw_index_of_visible(pos)?;
        let op = TextOp::Insert {
            id: Id::new(self.replica, self.sv.get(self.replica)),
            origin_left: i.checked_sub(1).map(|j| self.items[j].id),
            origin_right: self.items.get(i).map(|it| it.id),
            content: text.to_owned(),
        };
        match self.apply(&op) {
            Ok(Applied::New) => Some(op),
            _ => None,
        }
    }

    /// Delete `len` visible scalars from `pos`, apply, and return the
    /// operations (one per run of consecutive target ids).
    pub fn delete(&mut self, pos: usize, len: usize) -> Vec<TextOp> {
        let targets: Vec<Id> = self
            .items
            .iter()
            .filter(|it| !it.deleted)
            .skip(pos)
            .take(len)
            .map(|it| it.id)
            .collect();
        let mut ops = Vec::new();
        let mut k = 0;
        while k < targets.len() {
            let start = targets[k];
            let mut n = 1;
            while k + n < targets.len() && targets[k + n] == start.offset(n as u64) {
                n += 1;
            }
            let op = TextOp::Delete {
                id: Id::new(self.replica, self.sv.get(self.replica)),
                target: start,
                len: n as u64,
            };
            self.apply(&op).expect("local delete applies");
            ops.push(op);
            k += n;
        }
        ops
    }

    // ------------------------------------------------------------------
    // Remote edits
    // ------------------------------------------------------------------

    /// Apply one operation. Units already applied are verified and skipped;
    /// a gap or a missing dependency is a retryable error.
    pub fn apply(&mut self, op: &TextOp) -> Result<Applied, CollabError> {
        let id = op.id();
        let len = op.len();
        if len == 0 {
            return Err(CollabError::Malformed("empty operation"));
        }
        let next = self.sv.get(id.replica);
        if id.counter > next {
            return Err(CollabError::MissingDependency(Id::new(id.replica, next)));
        }
        let dup = (next - id.counter).min(len);
        if dup > 0 {
            self.verify_duplicate(op, dup)?;
            if dup == len {
                return Ok(Applied::Duplicate);
            }
        }
        let op = if dup > 0 {
            op.without_prefix(dup)
        } else {
            op.clone()
        };
        match &op {
            TextOp::Insert {
                id,
                origin_left,
                origin_right,
                content,
            } => {
                for dep in [origin_left, origin_right].into_iter().flatten() {
                    self.require_scalar(*dep)?;
                }
                if let (Some(l), Some(r)) = (origin_left, origin_right) {
                    let (li, ri) = (self.index_of(*l).unwrap(), self.index_of(*r).unwrap());
                    if li >= ri {
                        return Err(CollabError::Malformed("origins out of order"));
                    }
                }
                if self.bytes + content.len() > limits::MAX_DOCUMENT_BYTES {
                    return Err(CollabError::DocumentFull);
                }
                for (k, ch) in content.chars().enumerate() {
                    let k = k as u64;
                    let item = Item {
                        id: id.offset(k),
                        origin_left: if k == 0 {
                            *origin_left
                        } else {
                            Some(id.offset(k - 1))
                        },
                        origin_right: *origin_right,
                        ch,
                        deleted: false,
                    };
                    self.integrate(item);
                }
                self.bytes += content.len();
            }
            TextOp::Delete { id, target, len } => {
                for k in 0..*len {
                    self.require_scalar(target.offset(k))?;
                }
                for k in 0..*len {
                    let i = self.index_of(target.offset(k)).unwrap();
                    self.items[i].deleted = true;
                    self.deletes.insert(id.offset(k), target.offset(k));
                }
            }
        }
        let end = op.id().counter + op.len();
        self.sv.set(op.id().replica, end);
        self.log.push(op);
        Ok(Applied::New)
    }

    /// `id` must name a scalar this document holds. A unit that is applied
    /// but is not a scalar (it is a deletion) can never become one.
    fn require_scalar(&self, id: Id) -> Result<(), CollabError> {
        if self.ids.contains(&id) {
            Ok(())
        } else if self.sv.contains(id) {
            Err(CollabError::Malformed("reference to a deletion unit"))
        } else {
            Err(CollabError::MissingDependency(id))
        }
    }

    fn verify_duplicate(&self, op: &TextOp, n: u64) -> Result<(), CollabError> {
        match op {
            TextOp::Insert {
                id,
                origin_left,
                origin_right,
                content,
            } => {
                for (k, ch) in content.chars().take(n as usize).enumerate() {
                    let k = k as u64;
                    let uid = id.offset(k);
                    let want_left = if k == 0 {
                        *origin_left
                    } else {
                        Some(id.offset(k - 1))
                    };
                    let ok = self.index_of(uid).is_some_and(|i| {
                        let it = &self.items[i];
                        it.ch == ch
                            && it.origin_left == want_left
                            && it.origin_right == *origin_right
                    });
                    if !ok {
                        return Err(CollabError::IdConflict(uid));
                    }
                }
            }
            TextOp::Delete { id, target, .. } => {
                for k in 0..n {
                    if self.deletes.get(&id.offset(k)) != Some(&target.offset(k)) {
                        return Err(CollabError::IdConflict(id.offset(k)));
                    }
                }
            }
        }
        Ok(())
    }

    /// `integrateYjsMod`, unit for unit.
    fn integrate(&mut self, item: Item) {
        let n = self.items.len();
        let left: isize = item
            .origin_left
            .map_or(-1, |id| self.index_of(id).unwrap() as isize);
        let right: usize = item.origin_right.map_or(n, |id| self.index_of(id).unwrap());
        let mut dest = (left + 1) as usize;
        let mut scanning = false;
        let mut i = dest;
        loop {
            if !scanning {
                dest = i;
            }
            if i == n || i == right {
                break;
            }
            let other = &self.items[i];
            let oleft: isize = other
                .origin_left
                .map_or(-1, |id| self.index_of(id).unwrap() as isize);
            let oright: usize = other
                .origin_right
                .map_or(n, |id| self.index_of(id).unwrap());
            if oleft < left {
                break;
            } else if oleft == left {
                if oright < right {
                    scanning = true;
                } else if oright == right {
                    if item.id.replica < other.id.replica {
                        break;
                    }
                    scanning = false;
                } else {
                    scanning = false;
                }
            }
            i += 1;
        }
        self.ids.insert(item.id);
        self.items.insert(dest, item);
    }

    // ------------------------------------------------------------------
    // Sync and positions
    // ------------------------------------------------------------------

    /// Every operation unit not covered by `sv`, in a causal order: what a
    /// replica at `sv` needs to catch up.
    pub fn diff(&self, sv: &StateVector) -> Vec<TextOp> {
        let mut out = Vec::new();
        for op in &self.log {
            let have = sv.get(op.id().replica);
            let start = op.id().counter;
            let end = start + op.len();
            if end <= have {
                continue;
            }
            out.push(if have > start {
                op.without_prefix(have - start)
            } else {
                op.clone()
            });
        }
        out
    }

    pub fn relative_position(&self, pos: usize, assoc: Assoc) -> RelativePosition {
        let vis = self.visible_ids();
        let anchor = match assoc {
            Assoc::After => pos.checked_sub(1).and_then(|p| vis.get(p).copied()),
            Assoc::Before => vis.get(pos).copied(),
        };
        RelativePosition { anchor, assoc }
    }

    /// The visible index a relative position stands for now, or `None` if
    /// its anchor is unknown here.
    pub fn resolve(&self, rp: RelativePosition) -> Option<usize> {
        let Some(anchor) = rp.anchor else {
            return Some(match rp.assoc {
                Assoc::After => 0,
                Assoc::Before => self.len(),
            });
        };
        let i = self.index_of(anchor)?;
        let before = self.items[..i].iter().filter(|it| !it.deleted).count();
        Some(match rp.assoc {
            Assoc::Before => before,
            Assoc::After => before + usize::from(!self.items[i].deleted),
        })
    }
}
