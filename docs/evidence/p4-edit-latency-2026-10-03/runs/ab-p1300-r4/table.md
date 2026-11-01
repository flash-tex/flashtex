**NON-REFERENCE run**: Low Power Mode on at start; Low Power Mode on at end; load1 up to 7.9 (> 5.0)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| full-100 | 101 | letter@start | 19 | 16.8 / 19.2 / 19.2 | p95 | 11 | 19.2 | 1230.9 | 0/19 | 99 / 99 | 405 MB | 7.94-7.33 | MISS: p95 19.2 ms > 11 ms |
| full-100 | 101 | letter@middle | 19 | 16.9 / 23.2 / 24.9 | p95 | 11 | 23.2 | 120.7 | 19/19 | 2 / 2 | 405 MB | 7.33-6.74 | MISS: p95 23.2 ms > 11 ms |
| full-100 | 101 | letter@end | 19 | 41.3 / 60.1 / 70.5 | p95 | 11 | 48.2 | 178.4 | 0/19 | 4 / 4 | 405 MB | 6.74-7.34 | MISS: p95 60.1 ms > 11 ms |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1). The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
