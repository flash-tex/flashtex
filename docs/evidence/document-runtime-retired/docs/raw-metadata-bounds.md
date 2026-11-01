# Raw metadata residency and request-derived bound proposal (FT049r8)

Current behavior rejects malformed metadata but retains `Vec<WireDocument>` before
checking the exact expected source count. A bounded synthetic probe using the same
raw Session constructor measured process peaks3748KB for an empty146-byte document
array frame,7528KB for50000 empty objects in150145 bytes, and4928KB for a524556-byte
frame carrying a512KB mismatching identity. Every case failed with no candidate.
These are process high-water observations, not exact allocator counts or timing.
The frame was below unchanged8MiB max_frame in every case. Evidence and runnable
probe are in `benchmarks/raw-metadata-residency` and `examples/raw_metadata_probe.rs`.

## Existing acceptance basis

`encode` already limits request id/project_id to128 UTF8 bytes, validates safe unique
paths and requires the entry snapshot, bounds source bytes and final encoded frame,
and restricts revisions to2^53−1. It imposes no independent document-count or path
length cap. The rendering-v2 schema specifies4096 documents/4096 path characters,
but that is not a new runtime-v1 request limit. Adopting those as a silent global
runtime cap would reject some currently accepted legacy requests.

The appropriate raw sibling budget is the actual submitted request: exact document
count, maximum/sum of its decoded path lengths, existing128-byte identity limits,
and the exact64-character SHA256 representation. These are necessary conditions for
successful correlation, not a guessed resource allowance. Byte lengths/hashes and
complete path-set equality still require the existing final source-binding check.
No new limits or implementation change is included in this proposal checkpoint.

## Early-bound strategy to review

The owner can make immutable request-derived metadata limits available before
writing a dispatched request, with the existing single permit serializing updates
before the next frame decode. It must retain the old request's limits through a
cancelled/stale accepted sibling and not replace them for queued work. No extra
queue/thread or JSON field-order assumption is needed.

Payload-before-discriminator and unsolicited/pre-v1 frames must not cause unlimited
metadata retention when no accepted-sibling budget is armed. A possible design is
bounded capture plus overflow/invalid markers: parse/type-check each source entry
with existing serde, retain only complete entries within the known request count,
and continue syntax validation without growing the metadata Vec. Reject markers
when the complete envelope classifies as v2. For v1, preserve existing unknown-field
handling and its full Value decoding instead of inventing a new documents cap.
Before any active request, retain no source metadata; unsolicited output remains a
protocol failure. Long decoded metadata strings can be checked before allocating
owned copies via serde visitors; escaped-string scratch allocation remains bounded
by max_frame and must not be described as eliminated.

This design needs explicit duplicate/type/depth/finite-number checks even for
nonretained entries. Merely switching overflow values to weaker IgnoredAny skips
would change the refusal policy. Missing-field objects should be marked without
retaining an entire empty struct per entry. Malformed content can terminate early
once failure is established, but no partial candidate may escape. Required v1,
unknown fields, cancelled/stale siblings, mode epochs, output caps and joined
shutdown remain regression gates before accepting any implementation.

## Implemented request-bound capture (FT049 revision 9)

The raw decoder now snapshots the active request's document count and maximum
UTF-8 path length under the existing permit. Dispatch arms this budget before
sending the request, and idle dispatch clears it. Queuing a later request does
not replace the active request budget. The mutex is released before parsing.
Before any request, document retention is zero.

Serde seeds capture bounded strings and at most the requested number of complete
document records. Missing/oversize records set an invalid marker; the parser still
checks every entry's types, known duplicate keys, numeric range, syntax and depth.
Only final v2 classification refuses that marker. V1 still runs the existing Value
decode and preserves unknown documents arrays. No renderer 4096-path/document
limit is imposed; a 5004-byte request path regression test pins that distinction.
The raw frame cap still bounds scanning, serde escape scratch and transient key
strings; this is not a global RSS or allocation-count bound. Vec capacity may
round above the retained entry count. Existing four raw frames, one decoder permit
and one retained candidate remain unchanged.

Paired fixture hashes exactly match the previous measurement. Separate-process
VmHWM observations changed from 3748/7528/4928 KiB (zero docs / 50000 empty docs /
512-KiB ID) to 3620/3784/4060 KiB. These are small residency observations, not
calibrated allocator counts or native speed claims. Full evidence is in
`../benchmarks/raw-metadata-bounded`; rejected frames still produce no candidate.
The empty-record unit test directly proves zero retained entries after 50001
records. Permit and cancellation tests cover budget replacement and reset.

## Actual producer acceptance (FT049 revision 10)

`benchmarks/raw-bounded-producer` preserves compressed complete evidence for the
unchanged pinned `6e696616cca27a6bff19d88f7e0fd51e64458e81` producer. Its binary
SHA and all 77 previously pinned assets were reverified; no producer rebuild,
cap increase or repeated memory probe was performed. Requested, legacy, declined
and failed negotiation pass. All 12 small fresh/persistent case/cap combinations
have equal v1 results and exact raw sibling bytes where accepted, and their
semantic results/envelopes match the earlier Value-route evidence. Cancellation
followed immediately by a two-document request with a longer UTF-8 path accepts
only the current candidate and verifies both exact source digests and lengths.
The additional source is an uploaded snapshot; this test does not claim include
layout coverage. Existing 11pt fixture diagnostics remain in the evidence.

To reproduce with the pinned binary and existing assets, set
`FLASHTEX_RAW_REPLAY=1`, `FLASHTEX_REPLAY_PRODUCER` to the provenance binary,
`FLASHTEX_FONT_DIRS=/tmp/flashtex-render-reference-stage/fonts` and
`FLASHTEX_TFM_DIRS=/tmp/flashtex-official-lm-root/fonts/tfm/public/lm`. Optionally
set `FLASHTEX_REPLAY_OUTPUT` to an existing temporary directory plus `runtime.json`.
Run each exact ignored test in `real_display_producer`:

- `published_producer_negotiates_source_bound_candidates`
- `actual_incremental_producer_fresh_and_persistent_runtime_equivalence`
- `actual_raw_cancelled_request_then_changed_source_budget`

These prove transport/cache/source acceptance only, not native render authority,
visual fidelity, or typing-to-visible latency. The default route remains unchanged.
