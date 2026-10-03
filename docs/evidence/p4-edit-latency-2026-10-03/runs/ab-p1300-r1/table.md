**NON-REFERENCE run**: on battery at start (100 %); Low Power Mode on at start; on battery at end (98 %); Low Power Mode on at end; load1 up to 16.3 (> 5.0)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| full-100 | 101 | letter@start | 19 | 15.8 / 16.4 / 16.5 | p95 | 11 | 16.4 | 1150.1 | 0/19 | 99 / 99 | 363 MB | 16.3-11.76 | MISS: p95 16.4 ms > 11 ms |
| full-100 | 101 | letter@middle | 19 | 15.6 / 16.1 / 16.7 | p95 | 11 | 16.1 | 89.9 | 19/19 | 2 / 2 | 363 MB | 11.76-10.48 | MISS: p95 16.1 ms > 11 ms |
| full-100 | 101 | letter@end | 19 | 39.0 / 40.1 / 49.8 | p95 | 11 | 28.5 | 117.7 | 0/19 | 4 / 4 | 363 MB | 10.48-9.96 | MISS: p95 40.1 ms > 11 ms |
| full-1000 | 1002 | letter@middle | 19 | 24.8 / 34.5 / 35.5 | p95 | 11 | 34.5 | 584.8 | 19/19 | 33 / 33 | 1621 MB | 9.73-11.33 | MISS: p95 34.5 ms > 11 ms |
| full-1000 | 1002 | letter@end | 19 | 44.9 / 64.1 / 76.8 | p95 | 11 | 43.1 | 275.2 | 19/19 | 6 / 6 | 1621 MB | 11.33-10.84 | MISS: p95 64.1 ms > 11 ms |
| full-1000 | 1002 | letter@start | 19 | 21.0 / 22.2 / 28.3 | p95 | 11 | 22.2 | 205.7 | 19/19 | 2 / 2 | 1621 MB | 10.84-9.72 | MISS: p95 22.2 ms > 11 ms |
| plain-100 | 100 | letter@end | 19 | 14.1 / 18.7 / 24.5 | p95 | 11 | 15.0 | 47.7 | 0/19 | 3 / 3 | 271 MB | 9.72-9.45 | MISS: p95 18.7 ms > 11 ms |
| plain-100 | 100 | letter@start | 19 | 9.3 / 11.5 / 12.7 | p95 | 11 | 11.5 | 58.5 | 19/19 | 2 / 2 | 271 MB | 9.45-9.73 | MISS: p95 11.5 ms > 11 ms |
| plain-100 | 100 | letter@middle | 19 | 14.7 / 20.5 / 60.6 | p95 | 11 | 20.5 | 315.9 | 19/19 | 33 / 33 | 271 MB | 9.73-9.23 | MISS: p95 20.5 ms > 11 ms |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1). The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
