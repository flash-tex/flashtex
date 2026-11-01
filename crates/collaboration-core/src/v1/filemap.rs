//! The project's file tree: a map CRDT keyed by a stable, random 128-bit
//! [`FileId`], so a rename never races an edit (the text CRDT is keyed by
//! `FileId`, not by path). Each field is a last-writer-wins register stamped
//! `(lamport, replica)`; the kind is fixed at creation. Proposal §2.6.

use std::collections::{BTreeMap, HashMap};

use super::{limits, Applied, CollabError, Fnv64, Id, StateVector};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileId(pub [u8; 16]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Text = 0,
    Blob = 1,
}

/// A content-addressed binary file version (a figure, a font).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobRef {
    pub sha256: [u8; 32],
    pub bytes: u64,
    pub media_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileOpKind {
    Create { kind: FileKind, path: String },
    SetPath(String),
    SetBlob(BlobRef),
    SetDeleted(bool),
}

/// One file-map change; consumes one counter of its replica.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileOp {
    pub id: Id,
    pub lamport: u64,
    pub file: FileId,
    pub kind: FileOpKind,
}

/// `(lamport, replica)`: the last-writer-wins order. Unique per op because a
/// replica's lamport clock strictly increases.
type Stamp = (u64, u64);

#[derive(Debug, Clone)]
struct Entry {
    kind: FileKind,
    path: (Stamp, String),
    blob: Option<(Stamp, BlobRef)>,
    deleted: (Stamp, bool),
}

/// A live file as the user sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntryView {
    pub file: FileId,
    /// The path it is materialised at: its own, or a conflict name.
    pub path: String,
    /// The path its register holds.
    pub requested_path: String,
    pub kind: FileKind,
    pub blob: Option<BlobRef>,
    /// True if another live file holds `requested_path`.
    pub conflict: bool,
}

/// A valid project path: relative, `/`-separated, no empty, `.` or `..`
/// segment, no backslash or NUL, at most [`limits::MAX_PATH_BYTES`].
pub fn is_valid_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= limits::MAX_PATH_BYTES
        && !path.contains('\\')
        && !path.contains('\0')
        && path
            .split('/')
            .all(|seg| !seg.is_empty() && seg != "." && seg != "..")
}

/// `dir/stem (conflict <replica as 16 hex>-<lamport>).ext`: unique, because
/// the stamp is.
pub fn conflict_name(path: &str, stamp_lamport: u64, stamp_replica: u64) -> String {
    let (dir, name) = match path.rfind('/') {
        Some(i) => (&path[..=i], &path[i + 1..]),
        None => ("", path),
    };
    let tag = format!(" (conflict {stamp_replica:016x}-{stamp_lamport})");
    match name.rfind('.') {
        Some(d) if d > 0 => format!("{dir}{}{tag}{}", &name[..d], &name[d..]),
        _ => format!("{dir}{name}{tag}"),
    }
}

#[derive(Debug, Clone)]
pub struct FileMap {
    replica: u64,
    lamport: u64,
    entries: BTreeMap<FileId, Entry>,
    ops: HashMap<Id, FileOp>,
    sv: StateVector,
    log: Vec<FileOp>,
}

impl FileMap {
    pub fn new(replica: u64) -> Self {
        Self {
            replica,
            lamport: 0,
            entries: BTreeMap::new(),
            ops: HashMap::new(),
            sv: StateVector::default(),
            log: Vec::new(),
        }
    }

    pub fn state_vector(&self) -> StateVector {
        self.sv.clone()
    }

    pub fn contains(&self, file: FileId) -> bool {
        self.entries.contains_key(&file)
    }

    pub fn kind(&self, file: FileId) -> Option<FileKind> {
        self.entries.get(&file).map(|e| e.kind)
    }

    pub fn is_deleted(&self, file: FileId) -> Option<bool> {
        self.entries.get(&file).map(|e| e.deleted.1)
    }

    fn local(&mut self, file: FileId, kind: FileOpKind) -> Result<FileOp, CollabError> {
        let op = FileOp {
            id: Id::new(self.replica, self.sv.get(self.replica)),
            lamport: self.lamport + 1,
            file,
            kind,
        };
        self.apply(&op)?;
        Ok(op)
    }

    pub fn create(
        &mut self,
        file: FileId,
        kind: FileKind,
        path: &str,
    ) -> Result<FileOp, CollabError> {
        self.local(
            file,
            FileOpKind::Create {
                kind,
                path: path.to_owned(),
            },
        )
    }

    pub fn rename(&mut self, file: FileId, path: &str) -> Result<FileOp, CollabError> {
        self.local(file, FileOpKind::SetPath(path.to_owned()))
    }

    pub fn set_blob(&mut self, file: FileId, blob: BlobRef) -> Result<FileOp, CollabError> {
        self.local(file, FileOpKind::SetBlob(blob))
    }

    pub fn set_deleted(&mut self, file: FileId, deleted: bool) -> Result<FileOp, CollabError> {
        self.local(file, FileOpKind::SetDeleted(deleted))
    }

    pub fn apply(&mut self, op: &FileOp) -> Result<Applied, CollabError> {
        let next = self.sv.get(op.id.replica);
        if op.id.counter < next {
            return if self.ops.get(&op.id) == Some(op) {
                Ok(Applied::Duplicate)
            } else {
                Err(CollabError::IdConflict(op.id))
            };
        }
        if op.id.counter > next {
            return Err(CollabError::MissingDependency(Id::new(op.id.replica, next)));
        }
        if op.lamport == 0 {
            return Err(CollabError::Malformed("lamport 0"));
        }
        let stamp = (op.lamport, op.id.replica);
        match &op.kind {
            FileOpKind::Create { kind, path } => {
                if !is_valid_path(path) {
                    return Err(CollabError::Malformed("invalid path"));
                }
                if self.entries.contains_key(&op.file) {
                    return Err(CollabError::IdConflict(op.id));
                }
                self.entries.insert(
                    op.file,
                    Entry {
                        kind: *kind,
                        path: (stamp, path.clone()),
                        blob: None,
                        deleted: (stamp, false),
                    },
                );
            }
            other => {
                let Some(entry) = self.entries.get_mut(&op.file) else {
                    return Err(CollabError::UnknownFile(op.file));
                };
                match other {
                    FileOpKind::SetPath(path) => {
                        if !is_valid_path(path) {
                            return Err(CollabError::Malformed("invalid path"));
                        }
                        if stamp > entry.path.0 {
                            entry.path = (stamp, path.clone());
                        }
                    }
                    FileOpKind::SetBlob(blob) => {
                        if entry.kind != FileKind::Blob {
                            return Err(CollabError::Malformed("blob on a text file"));
                        }
                        if blob.media_type.len() > limits::MAX_MEDIA_TYPE_BYTES {
                            return Err(CollabError::Malformed("media type too long"));
                        }
                        if entry.blob.as_ref().is_none_or(|(s, _)| stamp > *s) {
                            entry.blob = Some((stamp, blob.clone()));
                        }
                    }
                    FileOpKind::SetDeleted(d) => {
                        if stamp > entry.deleted.0 {
                            entry.deleted = (stamp, *d);
                        }
                    }
                    FileOpKind::Create { .. } => unreachable!(),
                }
            }
        }
        self.lamport = self.lamport.max(op.lamport);
        self.sv.set(op.id.replica, next + 1);
        self.ops.insert(op.id, op.clone());
        self.log.push(op.clone());
        Ok(Applied::New)
    }

    pub fn diff(&self, sv: &StateVector) -> Vec<FileOp> {
        self.log
            .iter()
            .filter(|op| !sv.contains(op.id))
            .cloned()
            .collect()
    }

    /// Live files at their materialised paths, sorted by path. When live
    /// files share a path, the one whose path register has the lowest stamp
    /// keeps it and every other gets [`conflict_name`]. Nothing is dropped.
    pub fn files(&self) -> Vec<FileEntryView> {
        let mut by_path: BTreeMap<&str, Vec<(Stamp, FileId)>> = BTreeMap::new();
        for (id, e) in &self.entries {
            if !e.deleted.1 {
                by_path.entry(&e.path.1).or_default().push((e.path.0, *id));
            }
        }
        let mut out = Vec::new();
        for (path, mut group) in by_path {
            group.sort();
            let conflict = group.len() > 1;
            for (k, (stamp, id)) in group.into_iter().enumerate() {
                let e = &self.entries[&id];
                out.push(FileEntryView {
                    file: id,
                    path: if k == 0 {
                        path.to_owned()
                    } else {
                        conflict_name(path, stamp.0, stamp.1)
                    },
                    requested_path: path.to_owned(),
                    kind: e.kind,
                    blob: e.blob.as_ref().map(|(_, b)| b.clone()),
                    conflict,
                });
            }
        }
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out
    }

    /// FNV-1a 64 over every entry (deleted ones included) in `FileId` byte
    /// order; the field layout is in `docs/contracts/collab-v1.md` §5.
    pub fn digest(&self) -> u64 {
        let mut h = Fnv64::default();
        for (id, e) in &self.entries {
            h.bytes(&id.0);
            h.bytes(&[e.kind as u8]);
            h.u64(e.path.0 .0);
            h.u64(e.path.0 .1);
            h.u64(e.path.1.len() as u64);
            h.bytes(e.path.1.as_bytes());
            match &e.blob {
                None => h.bytes(&[0]),
                Some((s, b)) => {
                    h.bytes(&[1]);
                    h.u64(s.0);
                    h.u64(s.1);
                    h.bytes(&b.sha256);
                    h.u64(b.bytes);
                    h.u64(b.media_type.len() as u64);
                    h.bytes(b.media_type.as_bytes());
                }
            }
            h.u64(e.deleted.0 .0);
            h.u64(e.deleted.0 .1);
            h.bytes(&[u8::from(e.deleted.1)]);
        }
        h.0
    }
}
