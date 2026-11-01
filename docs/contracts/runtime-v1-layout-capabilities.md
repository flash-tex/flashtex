# Runtime-v1 negotiated layout capabilities

Status: authoritative additive extension, September 12, 2026. The base runtime-v1
contract remains unchanged for clients that do not request these capabilities.
This is not rendering-v2 activation or a claim of exact LaTeX PDF identity.

## Negotiation

A compile request may include `payload.layout_capabilities`, a duplicate-free list
of at most 16 nonempty capability strings (each at most 64 UTF-8 bytes). This
revision defines `rules-v1` and `font-hints-v1`. The compiler may accept a subset;
its compile_result includes the accepted requested capabilities in the same payload
field. An omitted field means none. Never accept or emit an unrequested capability.
Unknown requested capabilities are not accepted. A consumer requiring a capability
must inspect acceptance and explicitly report its absence; it must not guess support.

Request IDs, project IDs, revisions, source hashes/ranges, stale-response suppression,
request/output bounds and all base runtime-v1 obligations still apply. Negotiation
is per request, so cached/incremental results must bind the accepted capability set.
A late response from another request/revision never changes the current renderer mode.

## rules-v1

When accepted, page items may include:

```json
{"kind":"rule","x_pt":72,"y_pt":84,"width_pt":24,"height_pt":0.5,"source":{"path":"main.tex","start_byte":0,"end_byte":11}}
```

The example source range illustrates a document containing `\frac{1}{2}`; the
numbers are illustrative, not measured TeX geometry. Coordinates use the base
contract's page coordinate system: x rightward, y downward from the page top.
The rectangle's top-left corner is `(x_pt,y_pt)`; width and height are positive,
finite values. Coordinate/dimension magnitudes may not exceed 1,000,000 page units.
Do not reinterpret y as a text baseline or infer geometry from a glyph run. Source
is the actual generating construct's valid UTF-8 range in the named document.
Page item order defines paint order. Rule appearance is opaque black in the exported
white document unless a future explicit color capability is negotiated; dark preview
may transform display colors without changing export data.

A consumer that accepted rules-v1 must draw the rectangle or report an unsupported
layout error. Unknown primitive kinds must never be silently skipped. An old client
that did not request rules-v1 must not receive a rule primitive. Legacy fraction-bar
text conventions may remain only on that legacy route with explicit approximation
limitations; they cannot establish exact rule fidelity.

## font-hints-v1

When accepted, text items may carry:

```json
{"font":{"family":"Latin Modern Roman","weight":"normal","style":"italic"}}
```

`family` is a nonempty requested family name of at most 128 UTF-8 bytes, with no
control characters. `weight` is `normal` or `bold`; `style` is `normal` or `italic`.
Existing text coordinates, baseline, size and source fields remain required.
Absence means legacy font selection. If a requested face cannot be resolved, the
consumer must explicitly report substitution or inability to render; it must not
claim the requested metrics/font were preserved.

These hints fix ambiguous style intent but do NOT identify font bytes, original
GIDs, shaping, encoding or exact advances. They are insufficient for byte/pixel
parity acceptance. Rendering-v2 resource hashes/original GIDs/exact positions remain
the route to stronger fidelity guarantees; do not cast TFM character codes to GIDs.

## Producer/consumer migration gates

1. Compiler accepts the request capabilities and emits only negotiated shapes;
   tests cover empty/unknown/unrequested capabilities and incremental=clean output.
2. PDF and native consumers parse and draw typed rules, honor explicit font hints,
   report substitution, and reject unknown kinds with source-aware diagnostics.
3. Only after consumer tests pass may that client request the capabilities.
4. Exact-ID/revision/capability tests cover asynchronous out-of-order replies and
   rapid switches between legacy and extended requests. Legacy fixtures remain valid.
5. Record actual rule geometry/font-resource reference comparisons independently;
   a successful negotiation test does not prove exact PDF or native latency.
