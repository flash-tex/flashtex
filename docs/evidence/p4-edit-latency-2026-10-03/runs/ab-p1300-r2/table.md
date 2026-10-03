**NON-REFERENCE run**: Low Power Mode on at start; Low Power Mode on at end; load1 up to 14.0 (> 5.0)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| full-100 | 101 | letter@start | 19 | 16.2 / 18.6 / 66.6 | p95 | 11 | 18.6 | 1243.1 | 0/19 | 99 / 99 | 376 MB | 13.97-13.84 | MISS: p95 18.6 ms > 11 ms |
| full-100 | 101 | letter@middle | 19 | 15.6 / 19.7 / 24.6 | p95 | 11 | 19.7 | 92.2 | 19/19 | 2 / 2 | 376 MB | 13.84-13.06 | MISS: p95 19.7 ms > 11 ms |
| full-100 | 101 | letter@end | 19 | 39.6 / 40.8 / 41.4 | p95 | 11 | 29.4 | 104.8 | 0/19 | 4 / 4 | 376 MB | 13.06-11.98 | MISS: p95 40.8 ms > 11 ms |
| full-1000 | 1002 | letter@middle | 19 | 19.3 / 19.8 / 20.0 | p95 | 11 | 19.8 | 501.3 | 19/19 | 33 / 33 | 1601 MB | 11.98-8.14 | MISS: p95 19.8 ms > 11 ms |
| full-1000 | 1002 | letter@end | 19 | 43.2 / 44.5 / 55.1 | p95 | 11 | 35.0 | 226.8 | 19/19 | 6 / 6 | 1601 MB | 8.14-7.49 | MISS: p95 44.5 ms > 11 ms |
| full-1000 | 1002 | letter@start | 19 | 21.3 / 22.5 / 28.2 | p95 | 11 | 22.5 | 199.8 | 19/19 | 2 / 2 | 1601 MB | 7.49-6.96 | MISS: p95 22.5 ms > 11 ms |
| plain-100 | 100 | letter@end | 19 | 14.1 / 15.6 / 23.2 | p95 | 11 | 9.1 | 56.9 | 0/19 | 3 / 3 | 245 MB | 6.97-6.89 | MISS: p95 15.6 ms > 11 ms |
| plain-100 | 100 | letter@start | 19 | 9.3 / 10.7 / 10.9 | p95 | 11 | 10.7 | 60.8 | 19/19 | 2 / 2 | 245 MB | 6.89-6.29 | PASS |
| plain-100 | 100 | letter@middle | 19 | 14.7 / 16.0 / 25.3 | p95 | 11 | 16.0 | 299.9 | 19/19 | 33 / 33 | 245 MB | 6.29-6.26 | MISS: p95 16.0 ms > 11 ms |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1). The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
