//! `collab-v1`: the reference implementation (the oracle) of live
//! collaboration's data model, per `docs/design/live-collab/PROPOSAL.md`
//! and the normative contract `docs/contracts/collab-v1.md`.
//!
//! The product implementation is the pure-Swift `FlashTeXCollabCore`
//! (`apps/mac/Sources/FlashTeXCollabCore`), shared by the Mac and the iPad.
//! This module exists so that the Swift core can be checked against an
//! independent, deliberately simple implementation: both replay the same
//! `collab-v1` fixtures (`tests/fixtures/collab-v1/*.json`) and must reach
//! byte-identical text and an identical structure digest.
//!
//! The oracle trades speed for obviousness. A text document is one `Vec`
//! entry per Unicode scalar (tombstones included), every position is found
//! by a linear scan, and every insert is integrated one scalar at a time
//! with the FugueMax rule (Gentle's "YjsMod"; Weidner and Kleppmann,
//! *The Art of the Fugue*, 2023, show the two equivalent). The Swift core
//! stores runs in a B-tree and integrates a run at once; the fixtures are
//! what prove the two agree.
//!
//! The legacy `Document`/`Op` API at the crate root is untouched and is not
//! wire-compatible with this module.

pub mod filemap;
pub mod project;
pub mod text;
pub mod wire;

use std::collections::BTreeMap;
use std::fmt;

pub use filemap::{BlobRef, FileEntryView, FileId, FileKind, FileMap, FileOp, FileOpKind};
pub use project::{DocRef, Project, ProjectError, Section};
pub use text::{Assoc, RelativePosition, TextDoc, TextOp};

/// The id of one unit of an operation: the scalar an insert created, one
/// scalar's deletion, or one file-map change. Every replica numbers its
/// units `0, 1, 2, ...` with no gaps, per document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id {
    pub replica: u64,
    pub counter: u64,
}

impl Id {
    pub const fn new(replica: u64, counter: u64) -> Self {
        Self { replica, counter }
    }

    /// The id `n` units later in the same replica's sequence.
    pub fn offset(self, n: u64) -> Self {
        Self {
            replica: self.replica,
            counter: self.counter + n,
        }
    }
}

/// For each replica, the number of its units this document has applied,
/// which (because units apply in order) is also the next counter expected.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StateVector(pub BTreeMap<u64, u64>);

impl StateVector {
    pub fn get(&self, replica: u64) -> u64 {
        self.0.get(&replica).copied().unwrap_or(0)
    }

    pub fn set(&mut self, replica: u64, next: u64) {
        self.0.insert(replica, next);
    }

    /// True if the unit `id` is covered.
    pub fn contains(&self, id: Id) -> bool {
        id.counter < self.get(id.replica)
    }
}

/// Bounds shared by every document. The Swift core enforces the same ones.
pub mod limits {
    /// Largest text document, in UTF-8 bytes of visible plus deleted text
    /// (matches `edit-ledger::MAX_DOCUMENT_BYTES`).
    pub const MAX_DOCUMENT_BYTES: usize = 8 * 1024 * 1024;
    /// Most operations waiting in a pending buffer.
    pub const MAX_PENDING_OPS: usize = 65_536;
    /// Longest project path, in UTF-8 bytes.
    pub const MAX_PATH_BYTES: usize = 1024;
    /// Longest blob media type, in UTF-8 bytes.
    pub const MAX_MEDIA_TYPE_BYTES: usize = 255;
}

/// Why an operation was not applied. Every variant is a bounded,
/// non-panicking rejection; none leaves the document changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollabError {
    /// A unit this operation depends on is not applied yet: an earlier unit
    /// of the same replica (a gap), an origin, or a delete target. The
    /// operation can be retried once it is; [`project::Project`] buffers it.
    MissingDependency(Id),
    /// The operation's file is not in the file map yet; retry later.
    UnknownFile(FileId),
    /// A unit id already applied with different content. Ids are never
    /// reused (`id_reuse_divergence.rs`); a reuse is refused, not merged.
    IdConflict(Id),
    /// Structurally invalid: an empty insert, a zero-length delete, an
    /// origin pair out of order, an invalid path, a kind mismatch.
    Malformed(&'static str),
    /// The document would exceed [`limits::MAX_DOCUMENT_BYTES`].
    DocumentFull,
    /// The pending buffer is at [`limits::MAX_PENDING_OPS`].
    PendingFull,
}

impl CollabError {
    /// True if the operation may apply later, once more arrives.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            CollabError::MissingDependency(_) | CollabError::UnknownFile(_)
        )
    }
}

impl fmt::Display for CollabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for CollabError {}

/// Result of applying an operation that was not rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applied {
    /// New units were integrated (possibly after trimming a prefix that was
    /// already applied).
    New,
    /// Every unit was already applied with identical content.
    Duplicate,
}

/// 64-bit FNV-1a, the digest both implementations compute over a
/// document's structure (see [`TextDoc::digest`]). It detects divergence
/// in tests; it is not a security boundary.
#[derive(Debug, Clone, Copy)]
pub struct Fnv64(pub u64);

impl Default for Fnv64 {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Fnv64 {
    pub fn bytes(&mut self, data: &[u8]) {
        for &b in data {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    pub fn u64(&mut self, v: u64) {
        self.bytes(&v.to_le_bytes());
    }
}
