# Experimental whole-result chunk reassembly

Status: consumer prototype only. Production runtime-v1 and native protocol are
unchanged. Compiler/native owners must agree on a negotiated capability and its
begin/end envelopes before any producer or reader activation.

The caller constructs an Assembly with a bound Request (project, revision, ID,
source snapshot), announced result bytes and page count. Each JSON chunk repeats
project/revision/ID, a zero-based sequence and a nonempty UTF-8 string segment of
the original complete result JSON. The prototype supports at most64MiB total,
1MiB serialized chunk and10,000 pages. The caller selects a positive deadline up
to60seconds. It checks frame size before parsing and cumulative decoded bytes
before append. No allocation is made from an announced total alone.

No page is exposed before explicit finish. Finish requires exact announced byte
count, current revision, unexpired deadline, full existing runtime validation and
exact announced page count. Missing/duplicate/out-of-order chunks, wrong identity,
oversize, stale revision or cancellation terminally reject and clear the partial
buffer. The caller must cancel on document/epoch changes even if it reuses a numeric
revision; production Controller generations must remain monotonic. Failed stream
recovery needs a newly bound assembly; do not splice retries into old data.

Proposed future begin/end messages must bind a unique stream ID and a full-result
content hash, with bounded announced lengths. A producer cannot publish partial
page sets as a completed revision. Timeout or compiler failure retains the last
completed UI preview with an explicit stale/error marker, never half old/half new
pages. A transport disconnect aborts the unfinished assembly. The prototype has no
network authentication or producer negotiation; it must not be directly enabled
as a production wire protocol.

`cargo run --release --manifest-path crates/document-runtime/Cargo.toml --example
chunk_reassembly -- /tmp/single-request.json < /tmp/full-result.json` executes an
offline exact comparison. On the pinned compiler9026d8a500KB fixture,10,135,609
result bytes containing301pages reassembled exactly in155chunks. Largest chunk:
76,346bytes. JSON-string escaping increased wire payload to11,765,953bytes.
Packing/reassembling/validating took116.86ms in one Linux release run, excluding
compiler time and native painting. Peak RSS was not measured. The benchmark also
holds original JSON for comparison, so its memory is not consumer-only memory.

This prototype removes the single-frame requirement, not full-result allocation:
it retains a bounded byte buffer and parses the full result on completion. Parsed
JSON memory exceeds encoded bytes. It does not provide per-page memory residency,
compression, deltas, earlier paint, or lower latency. Next design comparison should
use typed page/item chunks and/or immutable page references to avoid JSON-string
escaping and permit bounded page staging with atomic final publication. Exact
reassembly and stale suppression remain mandatory acceptance gates.

## Typed page alternative

`PageAssembly` accepts an exact envelope header with an empty pages array, then
one `PageChunk` object per page. It checks identity, sequential page number,
announced count, serialized per-frame1MiB limit and cumulative64MiB wire budget.
All page objects remain private until finish applies the same production validator
directly to the assembled JSON value. Unknown fields are preserved. Stale input,
invalid order or oversize terminally clears the assembly. A single page larger
than1MiB is explicitly rejected; item subdivision remains future work.

On the same301-page fixture the typed alternative uses10,155,174wire bytes and
34,000bytes for its largest message. Borrowed page serialization avoids copying
all page values while packing. One paired release run measured101.08ms typed
packing/reassembly/validation versus117.44ms escaped-string chunks, both exactly
equal to the original. These are offline prototype measurements, not a native
latency improvement claim. Both retain the entire completed page set; the typed
consumer avoids the extra full JSON byte-buffer/parse on completion. Peak RSS,
producer streaming and final native acceptance remain unmeasured/unimplemented.

## Separate-process memory observation

The example's optional second argument is `strings`, `pages`, or `both` (default).
Run each mode separately under `/usr/bin/time -f 'max_rss_kib=%M'` to avoid one
prototype's retained allocations affecting the other's process high-water mark.
On the same301-page fixture, Linux reported247,840KiB for strings and237,736KiB
for pages; exact output equality passed in both. Corresponding packing/validation
times were114.47ms and100.79ms. This is one paired observation, not a statistical
memory benchmark. Both processes retain the full source JSON and parsed reference
for equality checking; the numbers must not be represented as consumer-only RSS.
The roughly10MiB reduction is consistent with eliminating the extra full JSON
byte buffer, but allocator behavior was not independently profiled.

## Provisional page sink and integrity completion

`push_to_sink` validates each page with the existing source-span/geometry rules
before invoking a borrowed `ProvisionalPage` callback tagged with request ID,
project and revision. It never marks the result complete. The sink may reject
admission; rejection clears the private assembly. A caller retaining copied pages
must enforce its own residency budget and clear all provisional content on error,
revision change or cancellation. The prototype caps100,000items per page and
1,000,000retained items overall, alongside the existing serialized wire limits.
`residency()` reports retained pages/items and admitted wire bytes, not heap size.

Provisional consumers must complete with `finish_verified(current_revision,
expected_digest)`. It returns the complete result only after full validation and
SHA256 agreement. Digest bytes are compact serde_json serialization with ordered
map keys, including the entire envelope. Producers must use the same encoding;
this is not a claim of general cross-language canonical JSON. The expected digest
must come from the matching producer completion record, not be recomputed from
received data and trusted as proof. Generic legacy `finish` remains available for
the earlier offline non-provisional prototype; it must not authorize provisional
export. No production native export is wired to either API.

SHA256 reuses the existing project-files implementation via an explicit local
path dependency. A64KiB buffered writer hashes serialization without constructing
a full extra serialized result. After buffering, the301-page offline prototype
reported first validated sink callback0.336ms and full packing/reassembly/validation
plus hash verification170.39ms, with67,298items retained. Unbuffered hashing had
made full completion294.19ms. Linux benchmark maxRSS238,024KiB includes the complete
reference and source bytes. Both runs exactly matched original JSON.

First-page timing begins AFTER the compiler's full output has already been read
and parsed for this offline test. It is not compiler first-page latency, streaming
producer evidence, native paint timing or proof of the200ms typing objective.
The expected reference digest computation is also outside the timed consumer
interval; full completion includes computing/checking the received result digest.
