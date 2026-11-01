# Experimental raw display candidate (FT049r5)

The default constructors and Value candidate stay unchanged.
`Session::spawn_command_raw_display_prototype(command, limits)` explicitly selects
a fixed raw decoding strategy for a new Session; candidate delivery is still OFF
until `set_display_candidates_enabled(true)`. Do not switch decoder strategies on
an in-flight session. Helper opt-in should create a new raw session with complete
snapshots and preserve its own source/session generations.

`take_current_raw_display_candidate()` moves one
`UntrustedRawDisplayCandidate`. Its immutable request_id/project_id/revision/sources
getters match the Value API; `raw()` borrows RawValue and `into_raw()` moves the
whole Box<RawValue>. Framing newline and exterior whitespace are excluded; every
byte within the JSON value is retained, including escaped keys, exponent spelling,
and opaque rendering duplicates. This is transport/source proof, not rendering
validation. Downstream must consume these raw bytes through its strict validator
without an intermediate Value conversion and recheck current epochs before paint.

## Validation and compatibility

Serde handles all tokenization: a full syntax visitor traverses without constructing
a Value tree and checks numeric finite decoding, strings and standard depth limits.
Typed envelope/payload/document structs reject duplicate known identity/source fields
(including escaped equivalent keys), integer coercions, wrong paths/revisions/hash/
lengths and duplicate document paths. A malformed or duplicate discriminator fails
rather than falling back to normalized Value. Unknown rendering fields remain exact
raw bytes for the existing renderer's stricter validation. Opaque geometry duplicates
are deliberately not declared valid here. Unknown nested unpaired Unicode surrogate
keys reject, matching Value; paired valid keys pass unchanged.

This is a stricter boundary than Value normalization for binding duplicates. Raw
exponent tokens also retain their spelling, so helper serialization/frame-admission
behavior may differ. No default compatibility or native activation is claimed.
Tests cover exact bytes, escape equivalence, opaque duplicates, duplicate bindings,
1e400, depth, trailing JSON, current/source epochs, oversize lines and missing/invalid
siblings. The existing four-raw-frame queue, one decoder permit and joined shutdown
remain unchanged; no new worker or queue is introduced. Raw and Value candidate
slots are mutually exclusive in actual decoder output and clear together.

## Measured tradeoff, not a speed promotion

`examples/raw_display_compare.rs` replays a pinned producer-derived1.15MB envelope
through separate processes. The fixture was compactly reserialized from the published
helper evidence, not claimed original wire bytes. Both modes produce equivalent JSON;
the raw mode additionally retains exact fixture bytes. Four alternating debug runs
recorded Value parsing652/974ms versus raw1359/1615ms on the loaded host, with
process high-water memory about18.2–18.4MB versus6.9–7.0MB before verification allocations. Exact profiles/provenance are
in `benchmarks/raw-display-prototype`. This demonstrates a concrete memory reduction
but a parse-time regression, not native responsiveness or a reason to enable by default.

The first published raw implementation made four full serde passes (discriminator,
syntax, typed metadata and RawValue construction). It retains raw bytes plus typed
metadata instead of a large Value tree. Original input storage and RawValue conversion
can coexist transiently; four queued raw frames and one retained candidate are extra.
No claim of one-buffer total memory, strict RSS cap or allocation-count measurement
is made. A future pass reduction needs the same syntax/duplicate/numeric/refusal gates;
unsafe unchecked JSON construction or a custom parser is not an acceptable shortcut.


## Two-pass internal refinement

The current implementation combines typed metadata extraction, duplicate checks,
full opaque-value syntax/finite/depth validation and envelope classification in one
existing-serde MapAccess traversal. It decides the route only after the complete
object is read; payload-before-type and escaped field names are supported. Unknown
values are traversed by the same checked Syntax visitor, never skipped through a
weaker JSON-number or depth gate. Required display fields are checked afterward.
Safe RawValue construction remains a second pass. V1 results are decoded as Value
on their established path after classification; default constructors are unchanged.
This is a serde visitor, not a custom JSON tokenizer, unsafe constructor or private
serializer-token shortcut. The public raw candidate API is unchanged.

In the paired fixture replay recorded in `benchmarks/raw-display-two-pass`, raw
parsing took41.6/39.5ms versus Value52.4/59.9ms; process peaks were6.8–6.9MB versus
18.2MB. This pair was run after the Commander measurement window cleared. It shows
a concrete benefit on this fixture while retaining exact bytes and semantic equality.
It is not a calibrated native latency result, and absolute times from the earlier
loaded-host four-pass experiment are not directly comparable. The fixture is still
producer-derived normalized JSON, not a claim of preserved original producer wire.

Refusal tests also cover payload-first field ordering, exact maximum interoperable
revision2^53−1, incorrect source lengths, unknown nested surrogate keys and existing
duplicate/numeric/depth/lifecycle/budget gates. In raw sessions, recognized metadata
keys are checked even on v1 classification; this stricter experimental behavior does
not change the default Value session. Downstream helper/render/native integration
must preserve original raw bytes and all current-source gates before activation.

## Required v1 overhead measurement

`benchmarks/raw-v1-overhead` extends the same captured-stream example with the
existing v1 ResponseProfile; no new corpus or production behavior is introduced.
The required107874-byte frame parsed in3.89/4.17ms under the default constructor
and6.77/6.04ms under the raw constructor. Validation stayed approximately0.43–0.54ms.
This pair shows about2–3ms additional required-frame work from raw classification's
metadata/syntax pass followed by Value decoding. It does not establish native
latency or predict cost on a larger frame.

An owner-to-decoder expected-frame hint could potentially avoid this work because
the single permit already prevents decoding the next frame until owner validation
finishes. Such a change needs careful policy review: raw classification currently
rejects duplicate recognized envelope/payload metadata even on a v1 result, whereas
the original Value path may normalize them. Simply switching hinted v1 frames to
Value would alter that stricter experimental refusal set. Any implementation must
preserve contiguous accepted-sibling handling through cancellation, mode epochs,
timeouts and malformed/unsolicited frames, without introducing another queue or
assuming JSON field order. No hint implementation or refusal change is included in
this measurement checkpoint.
