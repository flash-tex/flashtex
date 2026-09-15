# Proposal: `display-list-v2-only` — elide the runtime-v1 `pages` while a v2 sibling carries the frame

Revision r1, 2026-09-13 (lane mac-perf-3, FT-071). Status: **implemented
behind the opt-in capability** in `crates/render-pipeline` (producer) and
`apps/mac` (consumer); `protocol/rendering-v2.schema.json` and
`docs/contracts/runtime-v1*.md` are unchanged in meaning. Old consumers never
send the name and see byte-identical replies.

## Why

Every keystroke reply on the direct route is a runtime-v1 `compile_result`
(185 KB for HW1, 2.49 MB for the 60 KB body) followed by the negotiated
`display_list` sibling. With the v2 pane active the v1 `pages` items are
never painted: the pane, its caret sync and its click navigation all read the
sibling. The v1 payload is pure IPC weight (README of
`docs/evidence/perf-mac-2026-09-13T1740Z`, "For FT-070" item 2).

## Shape (additive)

- Request: `payload.layout_capabilities` gains `"display-list-v2-only"`. It
  is meaningful only next to `"display-list-v2"` (or the delta sibling,
  `docs/proposals/display-list-v2-delta.md`); a request listing it without
  `display-list-v2` is legal but it is never accepted.
- Reply: when — and only when — the reply carries a sibling line
  (`display_list` or `display_list_delta`), the `compile_result` payload's
  `pages` array is empty (`[]`), and the echoed `layout_capabilities` include
  `"display-list-v2-only"`. `status`, `diagnostics`, `revision`, `project_id`
  and the rest of the payload are exactly what the full reply carries; the
  sibling's bytes are unchanged by this capability.
- When the sibling is declined (`display_list_declined`, the existing size
  fallback) or the result is `failed`, the reply is today's: `pages` present,
  `"display-list-v2-only"` absent from the echo. The echo is therefore the
  consumer's only signal that pages were elided; it never infers from an
  empty array.

## Consumer rules (Mac shell, implemented)

- Sent per request while the v2 pane is active and `display-list-v2` is in
  the mode set; never part of `requestedLayoutCapabilities` (its presence or
  absence is not a mode switch and its non-acceptance is not a "missing
  capability" note).
- Leaving the v2 pane with an elided result re-requests the current revision
  with pages. `File > Export PDF` (the v1-layout export) refuses an elided
  result with a message pointing at Export Exact PDF or the v1 pane.
- `FLASHTEX_DISPLAY_V2_ONLY=0` stops requesting it.

## Measured (`ipc_bench.py --delta --only`, 20 warm keystrokes, this Mac under load)

| seed | v1 line before | v1 line after | v2 sibling (delta) |
| --- | ---: | ---: | ---: |
| HW1 (3 pages) | 185 667 B | 1 879 B | 259 481 B (was 1 157 718 B full) |
| demo (2 pages) | 222 541 B | 280 B | 807 836 B (was 1 945 350 B full) |
| body60k (~24 pages) | 2 494 207 B | unchanged: no sibling (declined > 16 MiB) | declined |
