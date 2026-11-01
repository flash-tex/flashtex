# Borrowed admission feasibility: source ownership is the boundary

Read helper80713a9e/current compile_current and runtime submit_internal/encode,
validate_reply_value, display_candidate and raw_display::Parsed::validate. No
production change or benchmark. Let S be total source UTF8 bytes and E the encoded
request bytes, including JSON escaping and newline. Helper clones S into owned
runtime InputDocuments before submit; encoding borrows those Strings into E.
Queued Pending retains both S+E plus paths/metadata. Dispatch moves E to the writer
but Pending retains S through v1 and any accepted sibling. Meanwhile the ledger
retains its source. At most one queued request per project plus one active request
exists; an active and newer queued snapshot may legitimately differ. Logical byte
accounting is not allocator capacity, RSS or elapsed-time measurement.

A merely borrowed Request cannot outlive submit or survive the next mutable ledger
edit. Borrowed serialization followed by cloning the same source only relocates
the S copy. Reusing encoded JSON as source would require decoding/indexing escapes
again; it does not provide original UTF8 boundaries directly. Arc-backed immutable
sources could share snapshots but require a ledger representation/API change and
copy-on-write rules, outside current runtime ownership.

## Narrow additive proposal

An opt-in borrowed admission API could serialize borrowed document text immediately
while constructing an owned validation proof per document: path, byte length,
raw-source SHA256 and exact UTF8 boundary membership. Current runtime source uses
are v1 range validation and v2 source hash/length checks; it does not render or
reparse retained source. ASCII sources need only an all-boundaries flag. General
UTF8 could use a bounded bitmap of length ceil((S+1)/8), with positions0 and S
included; retain exact membership, not approximate range acceptance. This trades
full retained S for proof bytes and moves SHA/boundary work earlier. It is not yet
measured, implemented or authorized for promotion. Per-document structures and
large document-count overhead must be included, not hidden by the bitmap estimate.

Serialize/prove/validate all input and limits before queue/latest/candidate mutation;
failed admission must leave prior requests intact. Keep current owned API behavior
and queued fairness/supersession unchanged. Proof belongs to the admitted request,
not the mutable caller; retain it through cancelled/stale sibling draining and
use it to derive existing decoder metadata budgets. Neither source text nor proofs
may be reconstructed from untrusted response declarations. Hashing alone is
insufficient: it cannot prove v1 UTF8 span boundaries.

Required gate before implementation choice: isolated representative admission copy
cost versus proof generation, source-size/document-count retained accounting,
ASCII/multibyte/empty/exact-end boundary equivalence to str::is_char_boundary,
all malformed range/source-set/hash checks, input mutation after return, failed
admission rollback, superseded/active snapshots and accepted-sibling cancellation.
No schema/wire/queue/thread/default-limit changes are implied. A concrete measured
benefit must outweigh earlier hashing/bitmap allocation; otherwise retain current
immutable owned snapshots. Helper adaptation waits for a tested additive API.
