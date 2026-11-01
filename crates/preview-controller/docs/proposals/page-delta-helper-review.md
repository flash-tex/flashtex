# Helper review: opt-in page/delta preview proposal

Status: review only. No new capability, wire format, page filtering, cache or source
authority is implemented or enabled by this document. Producer, runtime and Mac
consumer owners must agree a single schema before Commander integration.

Reviewed request: GitHub issue2 comments5646277113 and5646362851. The Mac lead
reports a 27-page producer with ~18ms constructing/serializing a4.3MB full reply,
and a separate560KB source producing18.2MB above the helper limit. These are owner
reports, not new measurements. The referenced `runtime-v1-display-list-v2.md` was
absent from inspected main and producer refs; resolve the authoritative contract
path/revision before implementation. Existing runtime-v1, helper display-forwarding
and actual strict rendering implementations remain the reviewed boundaries.

## Negotiation and immutable identity

Negotiate producer support and consumer reconstruction support independently, then
ACK helper enablement for this helper session. An advertised string alone does not
prove either endpoint implements reconstruction. Preserve unmodified full v1/v2
and decline behavior; do not replace a full preview with only visible pages.

A proposed transaction needs these concepts (names are not a wire specification):

| Identity | Required binding |
| --- | --- |
| Transport | helper session and negotiated epoch; reset invalidates every base |
| Document | project, entry point, complete document membership/generation |
| Source | each path's editor revision, byte count and exact content digest |
| Compile | target compile generation/request identity, distinct from editor revisions |
| Base | exact previously installed full snapshot digest and its full identity |
| Target | full ordered page count and target full-snapshot digest |
| Resources | complete target resource manifest and exact referenced byte identities |

Reuse requires the base actually installed by the consumer. Optional admission or
even a successful pipe write does not establish installation. The consumer must
ACK its validated base, or a request must explicitly declare a retained base that
both ends can prove. Never advance the producer's baseline merely because it sent
an optional frame: required output may evict a queued delta, and a started write
may fail. Every delta identifies its base independently; sequence adjacency is not
proof that a predecessor arrived.

## Atomic reconstruction and loss recovery

Prefer one installed base plus one bounded in-progress target, no unbounded chain.
A complete transaction states changed pages, removed pages, unchanged references,
and full target ordering. Reject duplicate IDs, missing pages, conflicting removal
and replacement, out-of-range indices or references outside the named base. A page
insert near the beginning can renumber the entire suffix; page index alone is not
a stable identity. Top-level metadata and diagnostics are complete target values,
not implicitly inherited from a previous successful compilation.

Construct and validate away from the UI thread. Atomically publish only after full
page/resource closure and strict rendering validation; recheck source/session/
membership/epoch immediately before adoption. Keep the installed base unchanged
on any refusal. An older frame may stay visibly stale; it cannot acquire current
source actions. Navigation/export still require a complete validated current state.

| Condition | Required result |
| --- | --- |
| Wrong/evicted base or digest mismatch | Refuse target; request one full resync |
| Lost optional transaction | Keep base; next transaction must name a retained base or resync |
| Restart/disable/epoch or project change | Clear negotiated state and bases; explicit enable + full baseline |
| Source changes during assembly | Discard target for current adoption; preserve durable edits |
| Missing/invalid resource, unsupported primitive | Explicit refusal; retain valid prior state; never substitute a font |
| Budget exceeded | Explicit bounded refusal; release target resources; no truncated success |
| Full resync too large for current transport | Explicit unsupported/refused result, not an endless resync loop |

Bound resync retries to one outstanding request for a target/base failure, cancel it
on a newer source epoch, and expose permanent capability/size refusal rather than
retrying forever. Do not create a second parser for existing full-list semantics.

## Digests, source ranges and resources

Define exactly which bytes the full and page digests cover. Semantic JSON equality
is not byte equality: key order, number spelling, resource order and frame wrapping
must be specified if reconstruction claims the producer's identical full output.
Retaining exact original page fragments still requires exact top-level serialization
and resource ordering. Validate reconstructed full output against an unchanged fresh
full compile, not against a second incremental implementation using the same cache.

A paint-only digest cannot authorize source reuse. Current page spans use path/byte
offsets while document revision/hash metadata is global; an edit can shift offsets
on an otherwise visually identical page. Rebuild target metadata and validate all
retained spans against current authoritative sources. Keep distinct paint reuse and
semantic/source identity if the producer proposes that optimization. Ligatures,
macro-origin ranges, diagnostics, links and logical caret geometry belong in the
full-equivalence gate, not just pixels.

Resource closure includes unchanged pages. Replacing an asset under the same name
must not reuse a verified font object: identity includes original bytes, length,
face/instance/glyph semantics and digest. An omitted resource may be collected only
after no installed/in-progress state references it. Document exact ownership of
font bytes, parsed programs, rendered pages and exporter state across commit/reset.

## Transport and retained-memory constraints

The current helper complete-frame limit remains16MiB, input limit1MiB, and producer
frame allowance includes envelope headroom. Required ACK/error/full-preview FIFO
and the single optional slot remain. A delta already writing cannot be preempted;
small page operations do not imply sub200ms responsiveness. Do not place a new
unbounded queue before the current bounded queue.

Rendering review reports a16MiB helper-binding limit, generic full parser32MiB,
<=256 source members, <=10,000 contiguous full-list pages and <=256 PDF export
pages. Proposal owners must reconcile endpoint limits explicitly; a parser ceiling
is not an agreed retention/transport budget or permission to increase another cap.

A small patch can expand into a large full target. Budget the installed base,
patch bytes, reconstruction buffers, parsed target, source metadata, verified
resources and in-flight output together; counting wire bytes alone is insufficient.
Specify hard byte/page/item/resource ceilings and checked arithmetic before
allocation. Account for peak old+new states and avoid cloning a full page graph
through every layer. Resource sharing must not bypass eviction/lifetime bounds.

Critically, keeping the existing full v1 reply on every edit leaves its serialization
and transport cost in place. A v2-only delta cannot claim to remove that cost or
solve the18.2MB full-v1 refusal. The owners must separately propose how a negotiated
route supplies a complete baseline/resync while preserving legacy fallback. Chunked
resync, if needed, is another bounded atomic transaction contract—not a cap increase
or permission to deliver only pages that fit. No such route is approved here.

## Review and acceptance sequence

1. Producer and Mac owners submit exact schema, baseline ACK/resync behavior,
   canonical identity and byte/retention accounting; runtime/helper review it.
2. Offline fixtures reconstruct full results across insert/delete/reorder, page
   count changes, bibliography/diagnostic-only changes, shifted source ranges and
   resource changes. Compare complete bytes/semantics as explicitly claimed.
3. Fault gates cover dropped/replaced/out-of-order transactions, stale bases,
   corruption, restart, unknown resources and aggregate budget exhaustion. Existing
   durable edits and required replies must survive every optional refusal.
4. Native tests validate real helper→reconstruction→paint, source navigation and
   full export with exact authoritative state, including the560KB refusal case.
5. Measure producer serialization, helper transport, native assembly/validation,
   memory peak and keypress-to-current-paint separately. Report fixture sizes,
   actual binary hashes and fallback behavior. No speedup claim from patch size alone.

Owner agreement and explicit reviewed handoff precede any implementation dispatch.

Review pins: helper branch base7a7fd4f2; inspected main `c364f838c8dc783064ae489e2bcf2c69172a1871`;
producer remote `9aaec57a019c6a0073419eeb3ec90f922f5b367c`. Rendering-side companion: bba26e82,
`crates/rendering-core/docs/handoffs/page-delta/README.md`. These are review inputs,
not claims that the proposal is integrated or these producer changes are activated.
