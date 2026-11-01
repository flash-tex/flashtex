# infdesc in the M5 Pro T7 run: the phases before the harness error

From `raw.tgz` (`raw/infdesc-*.jsonl`, dl3-keys' records), as t7.py's `summarise` and `typing_row` take them
(the first keystroke of an in-body phase is the warm-up; typing: keystrokes >= 1), `t7.stats`. The preamble
phase stopped with `dl3-keys: no prose line on one page` (fixed in this lane), so the run has no infdesc rows of its own.
Non-reference (load1 3.2-5.9 during these phases). Host share: COMPILE to the edited page's PAGE frame, ms.

| phase | n | p50 | p95 | max | DONE p95 |
|---|---:|---:|---:|---:|---:|
| letter@end | 19 | 80.0 | 83.5 | 85.4 | 172 |
| letter@middle | 19 | 77.2 | 80.9 | 81.0 | 171 |
| letter@start | 19 | 31.6 | 34.5 | 36.8 | 198 |
| newline@middle | 19 | 78.9 | 87.5 | 90.2 | 190 |
| preamble | 0 | - | - | - | - |
| sentence@middle | 19 | 83.3 | 90.2 | 90.3 | 8322 |
| split@middle | 19 | 83.7 | 92.9 | 93.0 | 7757 |
| typing@100ms | 39 | 47.4 | 145.2 | 149.3 | - |
| typing@150ms | 39 | 88.3 | 116.9 | 124.7 | - |
| typing@50ms | 39 | 86.4 | 261.6 | 344.7 | - |
| typing@60ms | 39 | 969.1 | 1993.8 | 2112.3 | - |
| typing@80ms | 39 | 64.1 | 96.2 | 99.8 | - |
