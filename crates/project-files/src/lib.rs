//! FlashTeX project file layer (original Rust; its only dependencies are
//! `unicode-normalization` and the `libc` bindings used by [`sys`]).
//!
//! - [`graph`]: normalized project graph from an entry file, following
//!   `\input`, `\include`, `\bibliography`, `\addbibresource` and
//!   `\includegraphics` with byte spans; cycle, missing-file and path-escape
//!   diagnostics; runtime-v1 `documents` export (entry first).
//! - [`scan`]: the light reference scanner used by the graph.
//! - [`path`]: [`ProjectPath`] normalization (relative, forward slashes,
//!   never escaping the root).
//! - [`sha256`](mod@sha256): hand-written SHA-256 for content identity.
//! - [`revision`]: per-file revision counters and a derived project revision.
//! - [`save`]: atomic saves with hash-checked conflict refusal.
//! - [`watch`]: snapshot/diff external-change detection and a polling helper.
//! - [`recovery`]: crash-recovery journal of unsaved buffers.
//! - [`json`]: the minimal JSON used by the journal and payload export.
//!
//! See `README.md` for guarantees, non-guarantees and the native integration
//! proposal.

pub mod graph;
pub mod json;
pub mod path;
pub mod recovery;
pub mod revision;
pub mod save;
pub mod scan;
pub mod sha256;
pub mod sys;
pub mod watch;

pub use graph::{
    Diagnostic, DiagnosticKind, DiscoverError, Document, Edge, FileKind, FileSource, Overlay,
    ProjectFile, ProjectGraph, Severity,
};
pub use path::{PathError, ProjectPath};
pub use recovery::{
    CurrentState, Listing, RecoveryEntry, RecoveryError, RecoveryJournal, RestoreCheck,
};
pub use revision::{FileRevision, RevisionTracker};
pub use save::{
    DEFAULT_READ_LIMIT, Expected, FileIdentity, LOCK_FILE, ProjectLock, ProjectRoot, Refused,
    RootedRead, SaveConflict, SaveConflictKind, SaveError, SaveReceipt, save_atomic,
    save_atomic_bytes,
};
pub use scan::{ByteSpan, Reference, ReferenceKind, scan_references};
pub use sha256::{
    Digest, Sha256, hex as sha256_to_hex, parse_hex as sha256_from_hex, sha256, sha256_hex,
};
pub use watch::{
    ChangeKind, Conflict, ConflictKind, Diff, ExternalChange, FileState, Poller, RootReplaced,
    Snapshot,
};
