# Published producer6e69661 reply sizing and runtime boundary

Read-only review of exact6e696616cca27a6bff19d88f7e0fd51e64458e81:
`crates/render-pipeline/src/protocol.rs:17`, `:229`, `:236`, `:263`,
`src/display.rs:300`, and `src/bin/flashtex-render.rs:99`.
No producer source, runtime cap or native behavior changes accompany this note.

## Independent lines

`FLASHTEX_MAX_REPLY_BYTES` accepts a positive integer and clamps it to16777216.
Missing, noninteger or zero values use that default. The value independently gates
the v2 sibling and the v1 compile_result; it is not their combined output budget.
The worker writes the complete v1 line first and then any accepted sibling.

For requested v2 and nonfailed v1, the producer first computes
`estimated_json_bytes()`. It skips v2 serialization when that estimate exceeds the
limit. Otherwise it serializes and performs an exact JSON byte-length check. Only
the latter check establishes actual emitted JSON size. The estimator charges fixed
per-font/document/page/item overhead plus text/glyph/cluster quantities. The code
calls it an upper bound; this review has not established that claim for every
allowed string/escaping/provenance shape. The subsequent exact check still rejects
an underestimated serialized line. Conservative overestimates may decline a line
that would actually fit; the recorded estimate alone cannot distinguish that case.

A v2 decline removes only `display-list-v2` from accepted capabilities, adds
`display_list_declined`, changes ok to recovered, and emits no sibling. Existing
v1 pages and diagnostics remain. Then the complete resulting v1 envelope is
serialized and checked against the same limit. If v1 exceeds it, the producer
returns a minimal failed result, no sibling, and removes v2 acceptance even if an
extra line was prepared earlier. The minimal failure envelope is NOT recursively
checked against a tiny configured limit; the prior1-byte test therefore exercises
failure semantics, not one-byte wire compliance.

## Consumer framing

The producer compares JSON String length, excluding the newline it writes later.
Document-runtime defaults to8388608 max_frame bytes INCLUDING newline and enforces
each received line independently. Its raw queue remains four frames plus one
decoder permit; a retained candidate is additional documented state. Consequently:

- A producer-accepted v2 line can exceed the runtime's lower default and fail the
  session, even though both components enforce their own limits correctly.
- At equal numerical caps, a producer line exactly at the cap still exceeds a
  newline-inclusive consumer budget by one byte.
- This protocol negotiates format capability, not a common maximum frame size.
  A launcher targeting this exact producer may lower its environment reply cap
  to fit the actual consumer framing budget; no cap is raised by this review.
- Runtime must wait for one contiguous sibling only when the validated result
  accepted it and is nonfailed. Decline must finish normally with v1 fallback.
  Oversized, truncated or missing promised siblings fail explicitly; they cannot
  be silently relabelled declined after the accepted result.

## Existing evidence and remaining question

The26-page fixture preserves complete v1 frames around1.75MB and declines v2 using
an estimated27152372 bytes. Complete v1 fresh/persistent bytes match. No emitted
v2 frame was available from that stream, so neither actual v2 size nor large-v2
runtime acceptance was established. The unchanged producer's optional `--v2 FILE`
output can serialize the retained display into a separate artifact for an exact
size check without raising stream caps. Such an additional replay is pending the
active measurement window's release; this note does not invent its result.


## Exact CLI artifact observation

The authorized single `--v2` run is now complete with the identical26-page source,
verified6e69661 binary/assets, and unchanged caps. The CLI artifact is25120854
bytes without newline (25120855 if framed). Its SHA256 is
`eeacaaf4cbec909311cd511d1bec31681a3d3e565f48d664dd78e4498b0ce25f`.
The preflight estimate27152372 exceeds actual JSON by2031518 bytes, about8.1%.
The estimate is conservative in this concrete case; both estimated and actual
sizes exceed16MiB. This is not evidence of a false decline or a universal estimate
bound. The exact artifact also exceeds runtime's8MiB framed default.

Stdout contains exactly one1745747-byte v1 line with26 pages, byte-identical to the
prior multi-page replay, no accepted v2 capability and the explicit decline warning.
The file artifact's complete document set, raw UTF8 hashes, lengths and revisions
match the request. Producing this file does not negotiate or deliver a sibling.
The two line budgets remain independent; adding v1's size to v2 would describe a
different, nonexistent combined cap. Newline accounting remains separately relevant.

`benchmarks/display-exact-size-6e69661/evidence.json` records exact command, source,
binary, asset, artifact and stdout hashes. The25MB artifact is retained at the
recorded temporary evidence path; the repository keeps concise reproducible evidence
instead of another full large display copy. No source, serializer, thread, default,
stream cap, font mapping or native acceptance change was made.
