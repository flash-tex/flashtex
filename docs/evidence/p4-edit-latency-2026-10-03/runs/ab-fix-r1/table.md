**NON-REFERENCE run**: on battery at start (98 %); Low Power Mode on at start; Low Power Mode on at end; load1 up to 22.9 (> 5.0)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| full-100 | 101 | letter@start | 19 | 16.4 / 17.4 / 17.5 | p95 | 11 | 17.4 | 1254.3 | 0/19 | 99 / 99 | 395 MB | 8.81-7.74 | MISS: p95 17.4 ms > 11 ms |
| full-100 | 101 | letter@middle | 19 | 15.9 / 19.6 / 19.8 | p95 | 11 | 19.6 | 80.1 | 19/19 | 2 / 2 | 395 MB | 7.74-7.01 | MISS: p95 19.6 ms > 11 ms |
| full-100 | 101 | letter@end | 19 | 22.5 / 24.1 / 24.7 | p95 | 11 | 12.2 | 86.7 | 0/19 | 4 / 4 | 395 MB | 7.01-6.55 | MISS: p95 24.1 ms > 11 ms |
| full-1000 | 1002 | letter@middle | 19 | 20.1 / 29.0 / 32.3 | p95 | 11 | 29.0 | 583.1 | 19/19 | 33 / 33 | 1388 MB | 6.55-22.92 | MISS: p95 29.0 ms > 11 ms |
| full-1000 | 1002 | letter@end | 19 | 22.7 / 25.7 / 27.5 | p95 | 11 | 15.4 | 212.8 | 19/19 | 6 / 6 | 1388 MB | 22.92-21.64 | MISS: p95 25.7 ms > 11 ms |
| full-1000 | 1002 | letter@start | 19 | 21.3 / 25.7 / 27.2 | p95 | 11 | 25.7 | 161.6 | 19/19 | 2 / 2 | 1388 MB | 21.64-19.16 | MISS: p95 25.7 ms > 11 ms |
| plain-100 | 100 | letter@end | 19 | 13.0 / 14.0 / 19.2 | p95 | 11 | 11.5 | 44.8 | 0/19 | 3 / 3 | 254 MB | 19.16-17.49 | MISS: p95 14.0 ms > 11 ms |
| plain-100 | 100 | letter@start | 19 | 9.5 / 11.4 / 11.5 | p95 | 11 | 11.4 | 60.1 | 19/19 | 2 / 2 | 254 MB | 17.49-16.65 | MISS: p95 11.4 ms > 11 ms |
| plain-100 | 100 | letter@middle | 19 | 9.8 / 10.8 / 14.8 | p95 | 11 | 10.8 | 278.4 | 19/19 | 33 / 33 | 254 MB | 16.65-13.97 | PASS |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1). The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.
