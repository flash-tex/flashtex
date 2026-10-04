//! A whole project: the file map plus one text CRDT per text file, a
//! bounded pending buffer for out-of-order delivery, and state-vector sync.

use std::collections::BTreeMap;

use super::filemap::{FileId, FileKind, FileMap, FileOp};
use super::text::{TextDoc, TextOp};
use super::{limits, Applied, CollabError, Fnv64, Id, StateVector};

/// Which document of a project a section or state vector is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DocRef {
    FileMap,
    Text(FileId),
}

/// Operations for one document, as carried by `update` and `sync_reply`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Section {
    FileMap(Vec<FileOp>),
    Text(FileId, Vec<TextOp>),
}

/// An operation refused for good, named by its id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectError {
    pub doc: DocRef,
    pub op: Id,
    pub error: CollabError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingOp {
    File(FileOp),
    Text(FileId, TextOp),
}

impl PendingOp {
    fn doc(&self) -> DocRef {
        match self {
            PendingOp::File(_) => DocRef::FileMap,
            PendingOp::Text(f, _) => DocRef::Text(*f),
        }
    }
    fn id(&self) -> Id {
        match self {
            PendingOp::File(op) => op.id,
            PendingOp::Text(_, op) => op.id(),
        }
    }
}

/// What a parked operation counts against
/// [`limits::MAX_PENDING_BYTES`]: its variable-length payload plus 64.
pub fn pending_weight_text(op: &TextOp) -> usize {
    64 + match op {
        TextOp::Insert { content, .. } => content.len(),
        TextOp::Delete { .. } => 0,
    }
}

pub fn pending_weight_file(op: &FileOp) -> usize {
    use super::filemap::FileOpKind;
    64 + match &op.kind {
        FileOpKind::Create { path, .. } | FileOpKind::SetPath(path) => path.len(),
        FileOpKind::SetBlob(b) => b.media_type.len(),
        FileOpKind::SetDeleted(_) => 0,
    }
}

fn pending_weight(op: &PendingOp) -> usize {
    match op {
        PendingOp::File(op) => pending_weight_file(op),
        PendingOp::Text(_, op) => pending_weight_text(op),
    }
}

#[derive(Debug, Clone)]
pub struct Project {
    replica: u64,
    files: FileMap,
    texts: BTreeMap<FileId, TextDoc>,
    /// Distinct parked operations (an exact redelivery is not parked twice).
    pending: Vec<PendingOp>,
    pending_bytes: usize,
}

impl Project {
    pub fn new(replica: u64) -> Self {
        Self {
            replica,
            files: FileMap::new(replica),
            texts: BTreeMap::new(),
            pending: Vec::new(),
            pending_bytes: 0,
        }
    }

    pub fn replica(&self) -> u64 {
        self.replica
    }

    pub fn files(&self) -> &FileMap {
        &self.files
    }

    pub fn text(&self, file: FileId) -> Option<&TextDoc> {
        self.texts.get(&file)
    }

    /// A text file's document, for local edits that need it directly
    /// (undo). Operations made through it must be broadcast like any other.
    pub fn text_mut(&mut self, file: FileId) -> Option<&mut TextDoc> {
        self.texts.get_mut(&file)
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    fn sync_text_docs(&mut self, op: &FileOp) {
        if let super::filemap::FileOpKind::Create {
            kind: FileKind::Text,
            ..
        } = op.kind
        {
            self.texts
                .entry(op.file)
                .or_insert_with(|| TextDoc::new(self.replica));
        }
    }

    // Local edits: each returns the section to broadcast.

    pub fn file_op(
        &mut self,
        f: impl FnOnce(&mut FileMap) -> Result<FileOp, CollabError>,
    ) -> Result<Section, CollabError> {
        let op = f(&mut self.files)?;
        self.sync_text_docs(&op);
        Ok(Section::FileMap(vec![op]))
    }

    pub fn insert(&mut self, file: FileId, pos: usize, text: &str) -> Option<Section> {
        let op = self.texts.get_mut(&file)?.insert(pos, text)?;
        Some(Section::Text(file, vec![op]))
    }

    pub fn delete(&mut self, file: FileId, pos: usize, len: usize) -> Option<Section> {
        let ops = self.texts.get_mut(&file)?.delete(pos, len);
        (!ops.is_empty()).then_some(Section::Text(file, ops))
    }

    // Remote edits.

    fn try_apply(&mut self, op: &PendingOp) -> Result<Applied, CollabError> {
        match op {
            PendingOp::File(op) => {
                let r = self.files.apply(op)?;
                self.sync_text_docs(op);
                Ok(r)
            }
            PendingOp::Text(file, op) => match self.texts.get_mut(file) {
                Some(doc) => doc.apply(op),
                None => Err(CollabError::UnknownFile(*file)),
            },
        }
    }

    /// Apply sections from a peer, in any order, with duplicates. What
    /// cannot apply yet waits in the pending buffer (bounded by count and by
    /// bytes) and is retried after every success. Returns the operations
    /// refused for good.
    pub fn receive(&mut self, sections: &[Section]) -> Vec<ProjectError> {
        let mut errors = Vec::new();
        let ops = sections.iter().flat_map(|s| match s {
            Section::FileMap(ops) => ops.iter().cloned().map(PendingOp::File).collect::<Vec<_>>(),
            Section::Text(f, ops) => ops
                .iter()
                .cloned()
                .map(|op| PendingOp::Text(*f, op))
                .collect(),
        });
        for op in ops.collect::<Vec<_>>() {
            match self.try_apply(&op) {
                Ok(Applied::New) => self.drain_pending(&mut errors),
                Ok(Applied::Duplicate) => {}
                Err(e) if e.is_retryable() => self.park(op, &mut errors),
                Err(error) => errors.push(ProjectError {
                    doc: op.doc(),
                    op: op.id(),
                    error,
                }),
            }
        }
        errors
    }

    fn park(&mut self, op: PendingOp, errors: &mut Vec<ProjectError>) {
        if self.pending.contains(&op) {
            return;
        }
        let w = pending_weight(&op);
        if self.pending.len() >= limits::MAX_PENDING_OPS
            || self.pending_bytes + w > limits::MAX_PENDING_BYTES
        {
            errors.push(ProjectError {
                doc: op.doc(),
                op: op.id(),
                error: CollabError::PendingFull,
            });
            return;
        }
        self.pending_bytes += w;
        self.pending.push(op);
    }

    fn drain_pending(&mut self, errors: &mut Vec<ProjectError>) {
        loop {
            let mut progressed = false;
            let mut keep = Vec::new();
            for op in std::mem::take(&mut self.pending) {
                match self.try_apply(&op) {
                    Ok(Applied::New) => {
                        progressed = true;
                        self.pending_bytes -= pending_weight(&op);
                    }
                    Ok(Applied::Duplicate) => self.pending_bytes -= pending_weight(&op),
                    Err(e) if e.is_retryable() => keep.push(op),
                    Err(error) => {
                        self.pending_bytes -= pending_weight(&op);
                        errors.push(ProjectError {
                            doc: op.doc(),
                            op: op.id(),
                            error,
                        });
                    }
                }
            }
            self.pending = keep;
            if !progressed {
                return;
            }
        }
    }

    // Sync.

    pub fn state_vectors(&self) -> Vec<(DocRef, StateVector)> {
        let mut out = vec![(DocRef::FileMap, self.files.state_vector())];
        for (f, t) in &self.texts {
            out.push((DocRef::Text(*f), t.state_vector()));
        }
        out
    }

    /// What a peer with `remote` state vectors is missing: the file map
    /// first (it creates the text documents), then each text file.
    pub fn diff(&self, remote: &[(DocRef, StateVector)]) -> Vec<Section> {
        let empty = StateVector::default();
        let sv_of = |r: DocRef| {
            remote
                .iter()
                .find(|(d, _)| *d == r)
                .map_or(&empty, |(_, sv)| sv)
        };
        let mut out = Vec::new();
        let ops = self.files.diff(sv_of(DocRef::FileMap));
        if !ops.is_empty() {
            out.push(Section::FileMap(ops));
        }
        for (f, t) in &self.texts {
            let ops = t.diff(sv_of(DocRef::Text(*f)));
            if !ops.is_empty() {
                out.push(Section::Text(*f, ops));
            }
        }
        out
    }

    /// Digest of the whole project: the file map's, then each text file's
    /// id and digest in `FileId` order.
    pub fn digest(&self) -> u64 {
        let mut h = Fnv64::default();
        h.u64(self.files.digest());
        for (f, t) in &self.texts {
            h.bytes(&f.0);
            h.u64(t.digest());
        }
        h.0
    }
}
