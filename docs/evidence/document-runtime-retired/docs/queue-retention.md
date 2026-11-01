# Queued request residency

Admission previously bounded encoded/source lengths but retained arbitrary caller
String and document Vec capacities. Tiny accepted requests could therefore retain
large reserves until dispatch/completion, despite unchanged `max_frame` and
`max_projects`. Deterministic owned-buffer accounting demonstrates this gap:

| Active + three queued requests | Before | Compacted |
| --- | --- | --- |
| Actual source bytes | 32 | 32 |
| Request string capacities | 4194380 | 108 |
| Document Vec slots | 64 | 4 |
| Queued encoded bytes | 504 | 504 |
| Queued encoded capacities | 768 | 504 |

Admission now converts accepted owned strings/vectors through boxed forms to
remove spare capacities. This includes request identifiers, source paths/text,
document vectors, encoded bytes, capabilities and optional snapshot origin.
Every validation and capacity/duplicate/revision check precedes compaction.
Failed admissions leave existing queue state unchanged. Compaction is additional
admission work; the existing encode timing does not include it.

The regression sequence covers duplicate/project-capacity/oversize refusal,
same-project superseding with the existing move-to-back fairness, closing an
active project while its wire reply must drain, replacing that freed project
slot, closing a queued project and session failure cleanup. All queued source
text and serialized request bytes remain exactly equal. Complete accounting
snapshots are in `../benchmarks/queue-retention`.

This is capacity accounting, not allocator/RSS measurement. It excludes allocator
headers, the OS pipe, pending events/results/candidates, map nodes and writer-owned
bytes. After dispatch, the active Pending's encoded Vec is empty because ownership
moves to the writer. The writer has one channel slot plus a possible current write;
no claim counts its memory as absent merely because it moved out of Pending.

At most `max_projects` distinct latest projects and queued requests are admitted,
plus one active request that may belong to a closed project awaiting drain. An
active project can also have one newer queued revision. Thus source snapshots can
remain for `max_projects + 1` requests. Default limits intentionally still permit
substantial aggregate logical data; this change imposes no new logical cap. Paths
and document counts are indirectly bounded by the accepted encoded frame, while
source length is checked explicitly. Rejected input serialization can allocate
before its final encoded-length check; this accepted-retention change does not
claim to solve arbitrary caller-input transient allocation.

## Added admission work

A prepared-input debug test isolates the work from request validation and JSON
serialization. For 1000 requests with 16-KiB source text (32-KiB caller reserve),
final observed snapshot compaction totaled0.709ms and encoded-buffer trimming
0.230ms, approximately0.000709/0.000230ms per request. Loop/collection overhead
is included. The initial snapshot-only observation0.453ms is preserved too.
These bounded observations are not calibrated throughput or typing-latency
guarantees; allocator behavior and ordinary input shapes affect the cost.

The logical admission policy, event sequence, active source identity, writer
channel and scheduling remain unchanged. The reduction removes unused capacity
from buffers already accepted into runtime ownership; it does not erase source
snapshots needed to validate an in-flight or recovered reply.
