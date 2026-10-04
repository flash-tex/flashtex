//! A whole project: the file map plus one text CRDT per text file, a
//! bounded pending buffer for out-of-order delivery, and state-vector sync.

use std::collections::BTreeMap;

use super::filemap::{FileId, FileKind, FileMap, FileOp};
use super::text::{TextDoc, TextOp};
use super::{limits, Applied, CollabError, Fnv64, StateVector};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectError {
    /// The operation was refused for good (not retryable).
    Rejected(CollabError),
    /// The pending buffer is full; the caller should resynchronise.
    PendingFull,
}

#[derive(Debug, Clone)]
enum PendingOp {
    File(FileOp),
    Text(FileId, TextOp),
}

#[derive(Debug, Clone)]
pub struct Project {
    replica: u64,
    files: FileMap,
    texts: BTreeMap<FileId, TextDoc>,
    pending: Vec<PendingOp>,
}

impl Project {
    pub fn new(replica: u64) -> Self {
        Self {
            replica,
            files: FileMap::new(replica),
            texts: BTreeMap::new(),
            pending: Vec::new(),
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
    /// cannot apply yet waits in the pending buffer and is retried after
    /// every success. Returns the operations refused for good.
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
                Err(e) if e.is_retryable() => {
                    if self.pending.len() >= limits::MAX_PENDING_OPS {
                        errors.push(ProjectError::PendingFull);
                    } else {
                        self.pending.push(op);
                    }
                }
                Err(e) => errors.push(ProjectError::Rejected(e)),
            }
        }
        errors
    }

    fn drain_pending(&mut self, errors: &mut Vec<ProjectError>) {
        loop {
            let mut progressed = false;
            let mut keep = Vec::new();
            for op in std::mem::take(&mut self.pending) {
                match self.try_apply(&op) {
                    Ok(Applied::New) => progressed = true,
                    Ok(Applied::Duplicate) => {}
                    Err(e) if e.is_retryable() => keep.push(op),
                    Err(e) => errors.push(ProjectError::Rejected(e)),
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
