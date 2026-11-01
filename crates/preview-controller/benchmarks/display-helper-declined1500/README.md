# Explicit producer output-budget handling

Pinned actual producer with FLASHTEX_MAX_REPLY_BYTES=1500, original fixture and two
normal harness edits. First two states return recovered v1 with explicit
`display_list_declined` warnings and no sibling. Third state exceeds even the v1
budget (reported 1506 bytes) and returns failed v1 with an error and no sibling.
The harness records those statuses rather than relabeling them successful display
output or waiting indefinitely for an absent candidate. All three helper v1 values
match fresh direct producer output; final source remains durable and reopens exactly.

This is deliberately constrained-budget failure evidence, not full document
rendering, dropped-page acceptance, or production throughput. Reproduce with
`--reply-limit 1500` and the same verified assets/binaries. Receipts are single
Python-side observations, not native painting.
