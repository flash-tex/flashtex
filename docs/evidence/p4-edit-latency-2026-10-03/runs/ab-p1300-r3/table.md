**NON-REFERENCE run**: Low Power Mode on at start; Low Power Mode on at end; load1 up to 13.3 (> 5.0)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| full-100 | 101 | letter@start | 19 | 17.2 / 41.2 / 42.5 | p95 | 11 | 41.2 | 1375.1 | 0/19 | 99 / 99 | 381 MB | 13.34-10.98 | MISS: p95 41.2 ms > 11 ms |
| full-100 | 101 | letter@middle | 19 | 15.6 / 21.3 / 25.3 | p95 | 11 | 21.3 | 90.3 | 19/19 | 2 / 2 | 381 MB | 10.98-10.42 | MISS: p95 21.3 ms > 11 ms |
| full-100 | 101 | letter@end | 19 | 39.6 / 41.4 / 42.0 | p95 | 11 | 29.1 | 102.5 | 0/19 | 4 / 4 | 381 MB | 10.42-9.49 | MISS: p95 41.4 ms > 11 ms |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1). The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
