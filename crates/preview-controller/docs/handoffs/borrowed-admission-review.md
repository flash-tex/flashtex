# Independent helper acceptance review for borrowed admission

Reviewed runtime proposal49edf117 against helper80713a9e and current tests.
No implementation, measured speedup or activation. Runtime owner owns the isolated
prototype; this review supplies discriminating fixtures, not a second prototype.

The existing session::malformed_unicode_source_is_never_delivered_to_preview tests
one fixed invalid range. It does not establish that an admitted source proof remains
bound to its original text after caller mutation. Use the following same-length pair:

| Source | UTF8 bytes | Valid boundaries | Range0..1 | Range2..3 |
| --- | --- | --- | --- | --- |
| A: éa | 3 | 0,2,3 | reject | accept |
| B: aé | 3 | 0,1,3 | accept | reject |

Build/retain the proof for A, mutate or release the caller's storage, then validate
both ranges against A's retained proof. Validate B independently. Length-only checks
cannot pass this gate accidentally. Equal-length ASCII A/B additionally require
distinct raw-source hashes even though every byte offset is a character boundary.
Include empty source0..0, exact-end zero-width spans, reversed/out-of-range spans,
a four-byte scalar, and decomposed combining text. A boundary inside a grapheme but
between valid scalars remains legal: do not replace UTF8 semantics with grapheme rules.

For lifecycle integration after an API exists, the existing helper test
edit_admission_tracks_queued_supersession_and_failed_admission provides deterministic
started/release file gates. Hold A active, submit B and then C, release A, and check
that queued B supersession and current C admission IDs remain exact. Do not require
an old A preview to be delivered merely to exercise validation; stale display must
stay suppressed. Test proof equivalence separately from stale-publication policy.
An oversized later admission still leaves the durable source saved, returns no
admission receipt for that edit, and cannot rewrite older request proof state.

Existing display_sibling tests cover raw source hashes, lengths and candidate
lifecycle; retain their rejection of duplicate/unknown paths and mismatched revision.
The isolated prototype needs its own explicit limits statement: proof tests are not
a real Session lifecycle test until production admission actually consumes proofs.

Performance acceptance must include borrowed serialization, proof construction,
source hashing and allocations, compared with current clone plus serialization.
Report retained logical bytes separately from allocator capacity/RSS, and initial
admission latency separately from later reply-validation cost. Small retained proof
size alone cannot establish lower typing-to-visible latency.
