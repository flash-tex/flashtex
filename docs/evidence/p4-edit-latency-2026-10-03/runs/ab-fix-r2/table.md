**NON-REFERENCE run**: Low Power Mode on at start; Low Power Mode on at end; load1 up to 25.6 (> 5.0)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| full-100 | 101 | letter@start | 19 | 16.8 / 22.6 / 24.7 | p95 | 11 | 22.6 | 1292.8 | 0/19 | 99 / 99 | 358 MB | 6.26-7.36 | MISS: p95 22.6 ms > 11 ms |
| full-100 | 101 | letter@middle | 19 | 18.7 / 54.2 / 54.6 | p95 | 11 | 54.2 | 310.4 | 19/19 | 2 / 2 | 358 MB | 7.36-16.51 | MISS: p95 54.2 ms > 11 ms |
| full-100 | 101 | letter@end | 19 | 30.5 / 104.9 / 375.2 | p95 | 11 | 31.0 | 555.4 | 0/19 | 4 / 4 | 358 MB | 16.51-25.61 | MISS: p95 104.9 ms > 11 ms |
| full-1000 | 1002 | letter@middle | 19 | 20.7 / 22.2 / 22.6 | p95 | 11 | 22.2 | 490.9 | 19/19 | 33 / 33 | 1630 MB | 25.61-24.51 | MISS: p95 22.2 ms > 11 ms |
| full-1000 | 1002 | letter@end | 19 | 22.9 / 23.7 / 29.1 | p95 | 11 | 14.7 | 166.8 | 19/19 | 6 / 6 | 1630 MB | 24.51-21.6 | MISS: p95 23.7 ms > 11 ms |
| full-1000 | 1002 | letter@start | 19 | 21.6 / 22.8 / 26.8 | p95 | 11 | 22.8 | 143.0 | 19/19 | 2 / 2 | 1630 MB | 21.6-20.43 | MISS: p95 22.8 ms > 11 ms |
| plain-100 | 100 | letter@end | 19 | 13.4 / 15.3 / 16.7 | p95 | 11 | 9.2 | 47.9 | 0/19 | 3 / 3 | 256 MB | 19.43-18.52 | MISS: p95 15.3 ms > 11 ms |
| plain-100 | 100 | letter@start | 19 | 9.4 / 10.8 / 11.0 | p95 | 11 | 10.8 | 62.5 | 19/19 | 2 / 2 | 256 MB | 18.52-16.68 | PASS |
| plain-100 | 100 | letter@middle | 19 | 10.2 / 10.9 / 11.0 | p95 | 11 | 10.9 | 301.9 | 19/19 | 33 / 33 | 256 MB | 16.68-15.49 | PASS |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1). The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
