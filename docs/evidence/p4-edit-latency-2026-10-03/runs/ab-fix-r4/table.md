**NON-REFERENCE run**: Low Power Mode on at start; Low Power Mode on at end; load1 up to 7.3 (> 5.0)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| full-100 | 101 | letter@start | 19 | 16.0 / 18.3 / 19.1 | p95 | 11 | 18.3 | 1152.0 | 0/19 | 99 / 99 | 400 MB | 7.34-7.17 | MISS: p95 18.3 ms > 11 ms |
| full-100 | 101 | letter@middle | 19 | 15.6 / 16.1 / 16.8 | p95 | 11 | 16.1 | 76.9 | 19/19 | 2 / 2 | 400 MB | 7.17-6.83 | MISS: p95 16.1 ms > 11 ms |
| full-100 | 101 | letter@end | 19 | 22.3 / 23.8 / 27.9 | p95 | 11 | 12.2 | 89.9 | 0/19 | 4 / 4 | 400 MB | 6.83-6.52 | MISS: p95 23.8 ms > 11 ms |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1). The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
