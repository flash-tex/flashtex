**NON-REFERENCE run**: Low Power Mode on at start; Low Power Mode on at end; load1 up to 9.5 (> 5.0)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| full-100 | 101 | letter@start | 19 | 17.1 / 20.9 / 35.4 | p95 | 11 | 20.9 | 1426.6 | 0/19 | 99 / 99 | 400 MB | 9.49-8.67 | MISS: p95 20.9 ms > 11 ms |
| full-100 | 101 | letter@middle | 19 | 16.6 / 18.5 / 19.6 | p95 | 11 | 18.5 | 81.5 | 19/19 | 2 / 2 | 400 MB | 8.67-8.57 | MISS: p95 18.5 ms > 11 ms |
| full-100 | 101 | letter@end | 19 | 22.5 / 26.1 / 38.2 | p95 | 11 | 12.9 | 99.9 | 0/19 | 4 / 4 | 400 MB | 8.57-8.28 | MISS: p95 26.1 ms > 11 ms |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1). The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
