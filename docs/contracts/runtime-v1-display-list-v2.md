# Runtime-v1 `display-list-v2`: complete negotiated sibling snapshots

Status: Commander consolidation draft for owner review. This records implemented
full-snapshot behavior; it does not activate a new mode or authorize page/delta
messages. Until adoption is recorded, existing runtime-v1 and layout-capability
contracts plus the pinned implementations below remain the compatibility baseline.

The historical proposal exists on `agent/mac-preview-v2/live` at
`9dbdb01f8d7e64268c9be46cc88f95adf211f2ee` under this same path. It was absent from
main. Its face-salted font digest and helper-exclusion statements are obsolete.
This consolidation corrects those statements rather than importing them as rules.

## Negotiation and complete reply transaction

Requests use the existing duplicate-free `payload.layout_capabilities` list.
`display-list-v2` is requested only by a consumer prepared to validate it. Accepted
capabilities are an explicitly requested subset, bound to that request/result.
The base request and complete runtime-v1 `compile_result` remain unchanged.

For an accepted `display-list-v2` capability and a nonfailed (`ok` or `recovered`)
result, the producer writes exactly one complete protocol-version-2 `display_list`
JSON line immediately after that result and before a later request's reply. A
failed result cannot promise a display sibling. A declined or unrequested
capability produces no sibling; ordinary full v1 behavior remains available.
An unsupported producer may decline by omitting the capability. The current
producer's size decline uses `display_list_declined` and a recovered result.
Its preflight estimate can decline before exact v2 serialization; an estimated
size is not a measured serialized size. Exact output is also checked when produced.

The sibling request ID, project ID and compile revision must equal the admitted
request and result. Declared document paths, byte lengths and raw UTF-8 SHA-256
bindings must match their admitted source snapshot. Document revisions in this
producer envelope are compile revisions; they are not individual editor revisions.
An accepted but missing sibling remains subject to the original request timeout.
Duplicate, interleaved, unsolicited, malformed or wrongly correlated siblings must
not become render candidates. Stale or cancelled work cannot acquire currentness
by arriving later. Runtime may emit the v1 Preview before validating its promised
sibling; a missing or malformed sibling can fail afterward. This is not atomic
render installation. Stale or cancelled accepted v1 work still drains and validates
its one promised sibling before the next dispatch. A transport failure must leave
durable source state intact.

The complete snapshot includes all ordered pages, with page numbers1..N, document
metadata, font declarations, required features, diagnostics and source/hit/caret
semantics. Visibility filtering and omission of unchanged pages are not defined by
this capability. Existing `--v2` file output is a separate producer interface: a
file from an unnegotiated request is not evidence of a promised stdout sibling.
For accepted requests, exact file/stream equivalence requires its own pinned gate;
do not generalize it across request options or producer versions.

## Rendering and source validation

Transport validation does not establish renderability. Consumers validate the
complete envelope, supported features, checked coordinates/counts, full resources
and actual UTF-8 source ranges before use. Unknown unsupported profiles must be
refused, never silently reshaped or omitted. Generic rendering-core validation is
not equivalent to its explicit `PipelineCff` adapter: the generic default profile
is static TrueType; `opentype-cff` uses the explicit CFF binder and immutable font
registry. Do not claim that the generic schema accepts that extension unchanged.

For supported file-backed fonts, a resource SHA-256 identifies the complete
original font bytes, with face
index and other declared identity checked separately. The historical
SHA-256(bytes || face-index) engine identifier is not the raw resource digest.
Original GIDs, compiler positions and declared source spans are retained; consumers
do not invent mappings or repair malformed metadata. Resource verification must
bind the bytes actually used by the renderer, not an earlier path read.
The producer also describes metrics-only `core14-afm` resources with zero byte
length and an AFM identity digest; these are not authenticated paintable font files.
The strict PipelineCff path accepts neither AFM resources nor face-salted aliases.
A native schema decoding a metrics-only descriptor does not establish that it can
resolve that descriptor for painting.

Every declared source is checked against supplied bytes. The standalone CFF binder
does not itself assert that its declared document list equals an arbitrary caller
map. Helper membership authority is checked separately as described below.
Source ranges contain paths/offsets; their revision identity comes from the global
source metadata and current caller state, not from the range alone.

Recovery policy is API-specific. The strict paired helper consumer refuses
`tfm_missing`, `required_metrics_unavailable` and error diagnostics in the v1
`compile_result` when TeX metrics are required. Standalone searchable CFF export
checks v2 display diagnostics and refuses errors but can export warning-only output.
Pairing does not compare the two diagnostic sequences. PipelineCff binding checks
v2 diagnostic provenance without itself rejecting error severity; export applies
that refusal. Do not infer a blanket bind or hit refusal from a v2-only error.
The native route has its own diagnostic display and preparation policy; it must
not be described as enforcing the Rust paired-helper or export severity policy
without a route-specific gate. None of these outcomes establishes reference fidelity.

## Helper boundary and currentness

The helper route is explicitly opt-in and default OFF. A client uses
`configure_display_candidates` with the exact capability and renderer-support
confirmation. Enabling itself enrolls the producer layout capability, preserves
other requested capabilities, invalidates the old submission and submits a fresh
compile; a separate capability request or second compile is not required. The
client awaits its configuration result. `enabled:true` may accompany a
`preview_error`: it acknowledges policy, not producer acceptance, renderability or
a current candidate. Layout configuration cannot request display-list-v2 while
the mode is OFF.
See [display forwarding](../../crates/preview-controller/docs/display-forwarding.md)
and [raw transport](../../crates/preview-controller/docs/display-forwarding.md#experimental-raw-helper-contract-explicit-opt-in-only)
for the implemented commands; raw and Value startup strategies have distinct
capability acknowledgements and cannot switch an already queued decoder strategy.
Restart retains the selected transport strategy but resets enablement.

A helper `display_candidate` is explicitly untrusted with source actions disabled.
Its session/project/request/compile identity, membership generation and complete
editor-version map must match fresh caller-owned state. The original display
source hashes and lengths are checked against those actual current bytes; editor
revisions must not be equated with the compile revision. The event cannot supply
its own authority. Existing export/read-only-hit APIs recheck current caller state
at use. Candidate receipt is neither installation acknowledgement nor paint proof.

Required acknowledgements, errors and v1 output preserve required FIFO priority.
Optional candidates may be replaced, evicted or refused. A started optional write
cannot be preempted. Output admission and successful write do not prove consumer
installation. Retained old frames must not acquire new source authority. Historical
completed snapshots are a separate, mutually exclusive optional mode in the helper;
they are not current candidates. Native fallback/paint order remains a separately
tested consumer policy, not a guarantee made by the binder.

## Bounds, compatibility evidence and extensions

Limits apply independently at each layer. Current producer per-line limits,
configurable runtime input limits, renderer pair/event limits and complete helper
JSONL limits are not interchangeable. The runtime's default producer-output frame bound is8 MiB, not a helper request
limit. Current helper stdin is bounded at1 MiB; serialized helper JSONL at16 MiB
including newline, with its configurable producer-frame ceiling at15 MiB to leave
wrapper headroom. The current helper launcher sets the child producer JSON budget
at most runtime frame capacity minus one newline byte at startup and restart,
preserving a stricter valid inherited budget. Tiny configured budgets retain the
producer minimal-failed-reply exception documented in the size review. This is
launcher policy, not a numeric wire negotiation or a helper wrapper-size guarantee.
Pair/helper-event validation is16 MiB; the generic parser's32 MiB
ceiling is not a helper transport allowance. Framing newline, wrapper metadata and numeric
reserialization may change counted size. Consult the pinned
[producer size review](../../crates/document-runtime/docs/producer-size-contract.md).
There is no negotiated common byte budget in this capability. Full output exceeding
a receiving bound is an explicit failure/refusal, not permission to truncate it.

Compatibility evidence includes actual producer65dbe7d/6e69661 full sibling
replays, runtime FT049 request/source/lifecycle gates, strict rendering-core pairing
and helper actual-capture gates. Those tests do not establish current Mac paint or
external-reference parity. Historical native results retain their source/tool/date
scope. Source baselines for this consolidation are main228c5fa1 and producer9aaec57a;
no experimental page/delta code is part of either full-snapshot contract review.

A page/delta extension requires a separately reviewed contract: exact installed
base acknowledgement, complete new source/resource/page reconstruction, defined
digest serialization, bounded old-plus-new residency, wrong-base refusal and a
full resync path. Keeping full v1 replies retains their cost and size limit. An
18.2 MiB full resync cannot be made to fit a16 MiB transport by silently omitting
pages. No extension is enabled by this document.
