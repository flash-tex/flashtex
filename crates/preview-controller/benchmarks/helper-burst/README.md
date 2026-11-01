# Continuous typing through the real helper

The driver sends20 full-source edits from a writer thread while the reader concurrently drains helper output. Each request uses the exact predicted prior revision/hash; every durable acknowledgement is verified against its source. A preview older than the latest delivered acknowledgement fails the run. The final preview must exactly equal a fresh compiler build; killing/reopening must recover the exact final source and revision. Initialization and initial preview complete before timed typing begins. No source or result fields are dropped to meet timing.

| Source target | Intended interval | Durable ACKs | Preview updates | Stale / superseded | Final preview after last send start |
| --- | --- | --- | --- | --- | --- |
|5KB|30ms|20/20|20|0 /0|14.508ms|
|50KB|30ms|20/20|1|11 /8|91.361ms|
|50KB|100ms|20/20|17|3 /0|127.976ms|

All3final results equal clean builds and all3kill/reopens preserve exact source. At50KB/30ms there was only a final preview: correct coalescing does NOT satisfy continuous live-preview responsiveness at this workload. This exposes compiler/whole-result turnaround as ongoing work rather than claiming success from final latency alone. The actual per-edit send intervals and pipe-write durations are in each artifact, with helper/compiler hashes. Exact source bytes differ slightly from target labels and are also recorded.

Helper source4db4a7a, pinned original compiler executable554c9e054f9159c01f8ec184d85dec86b83bcd5da7d69ea99a7bdb6416108322. Shared Linux machine, one run per case, no native painting/presentation, reference PDF or live provider inference measured. Run examples/helper_burst.py --helper HELPER --compiler COMPILER --size 50000 --edits 20 --interval-ms 30. Only temporary project directories are mutated. The writer has bounded input size/count, the reader has bounded frames and deadlines, and the helper is killed/reaped during cleanup.
