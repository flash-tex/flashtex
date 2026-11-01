# Independent sustained typing capture audit

FT049 revision25. Audited helper capture `06b2f7a8e11481b64e5affa2590f56aa348e6454` at
`crates/preview-controller/benchmarks/typing-burst-50kb/checked/` using original committed compressed records. No producer,
helper or throughput workload was rerun; no runtime production changes were needed.

Run from repository root with the original locally pinned assets available:

```sh
python3 crates/document-runtime/tools/audit_typing_burst.py
```

The script verifies 15 compressed/raw artifact hashes and
78 asset hashes, then writes `benchmarks/typing-burst-review/audit.json`.
Missing assets fail explicitly; the script does not download or replace evidence.

All20 permanent command IDs are unique. The exact UTF8 byte edit sequence preserves
50,000 bytes per snapshot and verifies each previous SHA, revision and removed text.
ACKs accept revisions2–21 in order. Actual producer requests match those exact source
snapshots. During the measured interval, 12 edited snapshots reached the producer:
11 completed intermediate revisions map one-to-one to stale notifications, and21
is the sole current preview. The8 missing intermediate snapshots map exactly to
superseded request IDs. Initial revision1 and postmeasurement retry22 are excluded
from those counts. The separately restarted producer receives the exact final source.

The final revision21 request is byte-identical to the recorded clean replay input;
the full parsed producer, clean replay and helper final results agree. Current-preview
checks require monotonic revision and revision at least the latest preceding ACK.
History contains20 permanent IDs. Old first-command retry returns command revision2
with current revision21; last-command retry returns21/21. Both preserve final SHA,
and reuse with changed semantics produces `command_id_conflict`.

This demonstrates exact captured source/receipt attribution and final result equality.
It does not demonstrate intermediate current previews, every-keystroke visibility,
native rendering, visual parity or a general latency guarantee. Only one current
preview was recorded. The initial less strict harness remains upstream; this audit
checks the strengthened `checked` capture specifically. No additional compile is
inferred from a durable retry receipt; actual captured retry22 is listed separately.

Validation: the audit script and `git diff --check` passed. Existing Rust code was
unchanged, so the runtime suite was not redundantly rerun for this evidence-only task.
