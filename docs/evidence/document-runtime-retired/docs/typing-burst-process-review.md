# Independent separate-process sender pair audit

FT049r29; pinned helper publication `c5518be3`. No producer/helper workload was
rerun and no runtime production code changed.

```sh
python3 crates/document-runtime/tools/audit_typing_burst.py --process-mode current
python3 crates/document-runtime/tools/audit_typing_burst.py --process-mode historical
```

Both audit modes verify original archive/compressed hashes, untouched provenance,
local asset hashes, captured script hashes,20 guarded UTF8 edits and exact ACK/source
history. Decoded sender base64 frames equal original requests.jsonl byte-for-byte.
All20 sender indices, order, target/send/flush records and provenance copies agree.
Current/historical commands differ only by explicit binding tokens; initial/final
source, clean request and clean full reply bytes match across modes. Helper/producer
identities, assets and harness hashes are identical across the pair.

Current mode: eight actual intermediate producer completions map to eight stale
notifications;11 edited requests were superseded. Current21 is the only preview.
Historical mode: original intermediate completions are2,5,9,12,15,18,20. Delivered
historical results are2,5,9,12,18,20, each bound to a unique original request/result
in the same proxy stream, exact source/token/project/revision and false current/
source-action flags. Twelve edited requests were superseded. Completed revision15
has no historical/stale delivery in the captured events; it is explicitly recorded
as an undelivered completed optional result, not counted as superseded or current.
This audit does not infer why it was omitted. Current21 remains the sole current
preview. Retry22 and restarted producer1 are excluded from burst accounting.

Observed maximum sender lateness: current0.105345ms, historical0.092085ms. These
are verified recorded timestamps from one sequential pair, not a cadence guarantee,
causal overhead measurement or native-render latency. Historical output can improve
observability while stale; it does not create authority for source actions or prove
continuous current-preview delivery.

The new f240daec fragmented-transfer test was reviewed read-only: two greater-than-
PIPE_BUF frames including UTF8 are read in317-byte fragments and checked for exact
concatenation/order and terminal reader. Owner test execution is not relabeled as
an independent run here. The parent still relies on bounded existing reads to detect
sender failure, as documented in scheduled-sender-review.md.

Validation: all four pinned artifact audit modes pass, with prior current/historical
JSON evidence unchanged, plus git diff --check. No extra throughput/corpus sweep.
