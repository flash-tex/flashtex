# Instrumented failed capture: independent prefix audit

FT049r33; exact upstream9f4a310528329eb08a43179341751de73073cfb3.
The overall run remains **failed_post_measurement_reopen**. This review neither
reruns the workload nor converts partial evidence into a passing end-to-end gate.

```sh
python3 crates/document-runtime/tools/audit_instrumented_prefix.py
```

The audit verifies17 archived compressed/raw hashes and captured script hashes.
It reconstructs20 exact guarded UTF8 edits, source fingerprints and ordered ACKs.
Original complete producer request/reply pairs bind all10 historical results and
sole current21 to their exact source snapshots. Numeric historical generations,
command binding tokens and disabled current/source-action flags agree.

All49 first-session output sequences have one admitted/dequeued/write_started/
write_finished record, immutable class/size/generation metadata, and corresponding
Client-wide receiver ordinal/length. No receiver telemetry was dropped. Exactly
three startup frames precede events.jsonl. Every subsequent event's original line
length and historical generation agrees with its output/receiver association.
The audit keeps helper and receiver clock domains separate and does not infer
native paint or causal stage costs from their durations.

The reopened proxy input log contains the exact final source; logging before
forwarding alone does not establish child receipt. Its captured output-log prefix
is94208 bytes with zero complete JSONL frames. An interrupted log write after a
complete readline can also produce such a prefix; it does not prove the producer
emitted an incomplete JSON frame. Its SHA256 is
fd25c7d5d69c89a95d2cab0c6d5c445c6c0cfea0bfb9aceb63594a190f4c4f40.
Successful reopened preview/document validation, clean final direct comparison and
normal successful-run provenance are absent. No claim is made that the partial
output was accepted or that any particular stage caused the timeout. Binary/source
identities in failure.json are declarations; this prefix audit verifies archived
bytes and scripts, not a new build or asset-provenance run.

Root's revised transport_trace_review.py was reviewed read-only. It now checks
immutable metadata and known unique lifecycle outcomes for every sequence, rejects
terminal/write contradictions, checks original event lengths, and refuses dropped
receiver coverage or missing terminal evidence for received frames. The independent
prefix audit here additionally establishes that this actual first session has all
four complete lifecycle records for every one of49 frames. A different incomplete
capture must remain incomplete; no missing outcome is synthesized.

Validation: audit script and git diff --check pass. No Rust production changes,
heavy tests, throughput reruns or extra producer executions were performed.
