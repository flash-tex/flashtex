**NON-REFERENCE run**: power state not recorded; load1 up to 12.9 (> 5.0)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| plain-10 | 10 | letter@start | 19 | 17.3 / 18.5 / 18.6 | p95 | 11 | 18.5 | 111.9 | 19/19 | 9 / 9 | 275 MB | 4.78-4.87 | MISS: p95 18.5 ms > 11 ms |
| plain-10 | 10 | letter@middle | 19 | 10.1 / 10.6 / 11.2 | p95 | 11 | 6.8 | 73.2 | 19/19 | 6 / 6 | 275 MB | 4.87-5.71 | PASS |
| plain-10 | 10 | letter@end | 19 | 7.2 / 7.8 / 7.9 | p95 | 11 | 7.8 | 23.8 | 0/19 | 1 / 1 | 275 MB | 5.71-6.05 | PASS |
| plain-10 | 10 | sentence@middle | 19 | 10.6 / 11.3 / 11.4 | p95 | 11 | 7.3 | 48.0 | 0/19 | 6 / 6 | 275 MB | 6.05-6.05 | MISS: p95 11.3 ms > 11 ms |
| plain-10 | 10 | newline@middle | 19 | 10.6 / 11.9 / 17.1 | p95 | 11 | 8.3 | 44.6 | 0/19 | 6 / 6 | 275 MB | 6.05-7.88 | MISS: p95 11.9 ms > 11 ms |
| plain-10 | 10 | split@middle | 19 | 10.0 / 10.9 / 11.0 | p95 | 11 | 7.0 | 45.7 | 0/19 | 6 / 6 | 275 MB | 7.88-7.73 | PASS |
| plain-10 | 10 | preamble | 5 | 96.2 / 97.5 / 97.5 | max | 400 | 97.5 | 137.2 | 0/5 | 10 / 10 | 275 MB | 7.73-8.95 | PASS |
| plain-10 | 10 | reopen: host's share, cold (spawn -> page 1) | 5 | 664.5 / 667.8 / 667.8 | max | (100, app) | - | - | - | - | 275 MB | 8.95-10.16 | report-only: max 667.8 ms > 100 ms |
| plain-10 | 10 | reopen: host already listening (not counted, decision 8) | 5 | 29.4 / 36.1 / 36.1 | max | (100, app) | - | - | - | - | 275 MB | 8.95-10.16 | report-only |
| full-10 | 11 | letter@start | 19 | 32.5 / 35.4 / 36.8 | p95 | 11 | 35.4 | 159.9 | 0/19 | 11 / 11 | 469 MB | 9.91-10.02 | MISS: p95 35.4 ms > 11 ms |
| full-10 | 11 | letter@middle | 19 | 18.0 / 22.3 / 23.1 | p95 | 11 | 13.4 | 103.6 | 19/19 | 4 / 4 | 469 MB | 10.02-9.37 | MISS: p95 22.3 ms > 11 ms |
| full-10 | 11 | letter@end | 19 | 36.3 / 37.7 / 39.1 | p95 | 11 | 29.0 | 75.2 | 0/19 | 3 / 3 | 469 MB | 9.37-9.13 | MISS: p95 37.7 ms > 11 ms |
| full-10 | 11 | sentence@middle | 19 | 18.9 / 23.0 / 23.4 | p95 | 11 | 13.8 | 80.8 | 0/19 | 7 / 7 | 469 MB | 9.13-8.56 | MISS: p95 23.0 ms > 11 ms |
| full-10 | 11 | newline@middle | 19 | 19.4 / 23.2 / 27.1 | p95 | 11 | 13.9 | 82.8 | 0/19 | 7 / 7 | 469 MB | 8.56-8.65 | MISS: p95 23.2 ms > 11 ms |
| full-10 | 11 | split@middle | 19 | 20.6 / 22.9 / 24.5 | p95 | 11 | 13.8 | 84.0 | 0/19 | 7 / 7 | 469 MB | 8.65-8.12 | MISS: p95 22.9 ms > 11 ms |
| full-10 | 11 | preamble | 5 | 421.1 / 428.1 / 428.1 | max | 400 | 428.1 | 528.6 | 0/5 | 11 / 11 | 469 MB | 8.12-8.43 | MISS: max 428.1 ms > 400 ms |
| full-10 | 11 | reopen: host's share, cold (spawn -> page 1) | 5 | 628.3 / 638.6 / 638.6 | max | (100, app) | - | - | - | - | 469 MB | 7.44-7.92 | report-only: max 638.6 ms > 100 ms |
| full-10 | 11 | reopen: host already listening (not counted, decision 8) | 5 | 102.8 / 105.4 / 105.4 | max | (100, app) | - | - | - | - | 469 MB | 7.44-7.92 | report-only: max 105.4 ms > 100 ms |
| plain-100 | 100 | letter@start | 19 | 7.3 / 12.4 / 15.6 | p95 | 11 | 12.4 | 61.6 | 19/19 | 2 / 2 | 926 MB | 7.44-8.89 | MISS: p95 12.4 ms > 11 ms |
| plain-100 | 100 | letter@middle | 19 | 12.7 / 14.4 / 15.1 | p95 | 11 | 14.4 | 229.3 | 19/19 | 33 / 33 | 926 MB | 8.89-8.39 | MISS: p95 14.4 ms > 11 ms |
| plain-100 | 100 | letter@end | 19 | 20.3 / 22.1 / 22.5 | p95 | 11 | 18.2 | 49.2 | 0/19 | 3 / 3 | 926 MB | 8.39-9.0 | MISS: p95 22.1 ms > 11 ms |
| plain-100 | 100 | sentence@middle | 19 | 12.8 / 14.8 / 15.0 | p95 | 11 | 14.8 | 212.0 | 0/19 | 50 / 50 | 926 MB | 9.0-8.08 | MISS: p95 14.8 ms > 11 ms |
| plain-100 | 100 | newline@middle | 19 | 13.2 / 15.6 / 16.4 | p95 | 11 | 15.6 | 223.9 | 0/19 | 50 / 50 | 926 MB | 8.08-7.45 | MISS: p95 15.6 ms > 11 ms |
| plain-100 | 100 | split@middle | 19 | 13.7 / 16.0 / 18.2 | p95 | 11 | 16.0 | 238.8 | 0/19 | 50 / 50 | 926 MB | 7.45-7.57 | MISS: p95 16.0 ms > 11 ms |
| plain-100 | 100 | preamble | 5 | 102.6 / 108.2 / 108.2 | max | 400 | 108.2 | 477.2 | 0/5 | 100 / 100 | 926 MB | 7.57-7.45 | PASS |
| plain-100 | 100 | reopen: host's share, cold (spawn -> page 1) | 5 | 543.7 / 554.0 / 554.0 | max | (100, app) | - | - | - | - | 926 MB | 7.06-7.33 | report-only: max 554.0 ms > 100 ms |
| plain-100 | 100 | reopen: host already listening (not counted, decision 8) | 5 | 27.2 / 27.6 / 27.6 | max | (100, app) | - | - | - | - | 926 MB | 7.06-7.33 | report-only |
| full-100 | 101 | letter@start | 19 | 15.5 / 22.6 / 23.2 | p95 | 11 | 22.6 | 1089.0 | 0/19 | 99 / 99 | 2410 MB | 7.06-5.42 | MISS: p95 22.6 ms > 11 ms |
| full-100 | 101 | letter@middle | 19 | 14.4 / 19.1 / 19.4 | p95 | 11 | 19.1 | 90.9 | 19/19 | 2 / 2 | 2410 MB | 5.42-5.12 | MISS: p95 19.1 ms > 11 ms |
| full-100 | 101 | letter@end | 19 | 39.0 / 40.1 / 51.7 | p95 | 11 | 30.2 | 102.5 | 0/19 | 4 / 4 | 2410 MB | 5.12-4.64 | MISS: p95 40.1 ms > 11 ms |
| full-100 | 101 | sentence@middle | 19 | 15.4 / 20.5 / 20.7 | p95 | 11 | 20.5 | 442.4 | 19/19 | 33 / 33 | 2410 MB | 4.64-4.21 | MISS: p95 20.5 ms > 11 ms |
| full-100 | 101 | newline@middle | 19 | 16.2 / 20.8 / 21.3 | p95 | 11 | 20.8 | 529.8 | 0/19 | 51 / 51 | 2410 MB | 4.21-4.08 | MISS: p95 20.8 ms > 11 ms |
| full-100 | 101 | split@middle | 19 | 16.3 / 21.2 / 21.8 | p95 | 11 | 21.2 | 524.3 | 0/19 | 51 / 51 | 2410 MB | 4.08-4.31 | MISS: p95 21.2 ms > 11 ms |
| full-100 | 101 | preamble | 5 | 429.0 / 445.5 / 445.5 | max | 400 | 445.5 | 1369.1 | 0/5 | 101 / 101 | 2410 MB | 4.31-4.62 | MISS: max 445.5 ms > 400 ms |
| full-100 | 101 | reopen: host's share, cold (spawn -> page 1) | 5 | 681.6 / 704.0 / 704.0 | max | (100, app) | - | - | - | - | 2410 MB | 4.89-6.77 | report-only: max 704.0 ms > 100 ms |
| full-100 | 101 | reopen: host already listening (not counted, decision 8) | 5 | 109.5 / 112.4 / 112.4 | max | (100, app) | - | - | - | - | 2410 MB | 4.89-6.77 | report-only: max 112.4 ms > 100 ms |
| plain-300 | 301 | letter@start | 19 | 9.0 / 12.2 / 21.5 | p95 | 11 | 12.2 | 132.2 | 19/19 | 5 / 5 | 2406 MB | 6.77-7.11 | MISS: p95 12.2 ms > 11 ms |
| plain-300 | 301 | letter@middle | 19 | 8.5 / 11.8 / 13.3 | p95 | 11 | 11.8 | 159.1 | 19/19 | 9 / 9 | 2406 MB | 7.11-7.09 | MISS: p95 11.8 ms > 11 ms |
| plain-300 | 301 | letter@end | 19 | 18.1 / 23.2 / 30.9 | p95 | 11 | 23.2 | 124.8 | 0/19 | 7 / 7 | 2406 MB | 7.09-7.22 | MISS: p95 23.2 ms > 11 ms |
| plain-300 | 301 | sentence@middle | 19 | 7.5 / 8.3 / 9.0 | p95 | 11 | 8.3 | 347.2 | 19/19 | 65 / 65 | 2406 MB | 7.22-7.12 | PASS |
| plain-300 | 301 | newline@middle | 19 | 8.5 / 11.4 / 12.7 | p95 | 11 | 11.4 | 656.6 | 0/19 | 151 / 151 | 2406 MB | 7.12-6.8 | MISS: p95 11.4 ms > 11 ms |
| plain-300 | 301 | split@middle | 19 | 9.3 / 13.9 / 18.9 | p95 | 11 | 13.9 | 867.3 | 0/19 | 151 / 151 | 2406 MB | 6.8-9.53 | MISS: p95 13.9 ms > 11 ms |
| plain-300 | 301 | preamble | 5 | 124.3 / 136.7 / 136.7 | max | 400 | 136.7 | 1685.0 | 0/5 | 301 / 301 | 2406 MB | 9.53-9.3 | PASS |
| plain-300 | 301 | reopen: host's share, cold (spawn -> page 1) | 5 | 580.8 / 649.3 / 649.3 | max | (100, app) | - | - | - | - | 2406 MB | 8.98-9.33 | report-only: max 649.3 ms > 100 ms |
| plain-300 | 301 | reopen: host already listening (not counted, decision 8) | 5 | 28.8 / 34.6 / 34.6 | max | (100, app) | - | - | - | - | 2406 MB | 8.98-9.33 | report-only |
| full-300 | 299 | letter@start | 19 | 14.1 / 21.6 / 21.6 | p95 | 11 | 21.6 | 158.7 | 19/19 | 3 / 3 | 6744 MB | 8.9-7.83 | MISS: p95 21.6 ms > 11 ms |
| full-300 | 299 | letter@middle | 19 | 37.8 / 89.3 / 114.6 | p95 | 11 | 89.3 | 187.8 | 19/19 | 2 / 2 | 6744 MB | 7.83-9.42 | MISS: p95 89.3 ms > 11 ms |
| full-300 | 299 | letter@end | 19 | 56.7 / 61.6 / 75.5 | p95 | 11 | 51.0 | 180.1 | 19/19 | 3 / 3 | 6744 MB | 9.42-9.22 | MISS: p95 61.6 ms > 11 ms |
| full-300 | 299 | sentence@middle | 19 | 36.0 / 42.1 / 65.0 | p95 | 11 | 42.1 | 572.5 | 19/19 | 33 / 33 | 6744 MB | 9.22-8.2 | MISS: p95 42.1 ms > 11 ms |
| full-300 | 299 | newline@middle | 19 | 36.9 / 41.7 / 57.5 | p95 | 11 | 41.7 | 1722.6 | 0/19 | 150 / 150 | 6744 MB | 8.2-7.51 | MISS: p95 41.7 ms > 11 ms |
| full-300 | 299 | split@middle | 19 | 36.8 / 40.1 / 40.7 | p95 | 11 | 40.1 | 1526.1 | 0/19 | 150 / 150 | 6744 MB | 7.51-5.51 | MISS: p95 40.1 ms > 11 ms |
| full-300 | 299 | preamble | 5 | 479.4 / 533.2 / 533.2 | max | 400 | 533.2 | 3427.6 | 0/5 | 299 / 299 | 6744 MB | 5.51-4.65 | MISS: max 533.2 ms > 400 ms |
| full-300 | 299 | reopen: host's share, cold (spawn -> page 1) | 5 | 634.5 / 641.8 / 641.8 | max | (100, app) | - | - | - | - | 6744 MB | 4.11-4.61 | report-only: max 641.8 ms > 100 ms |
| full-300 | 299 | reopen: host already listening (not counted, decision 8) | 5 | 106.5 / 109.0 / 109.0 | max | (100, app) | - | - | - | - | 6744 MB | 4.11-4.61 | report-only: max 109.0 ms > 100 ms |
| plain-1000 | 1001 | letter@start | 19 | 11.2 / 15.8 / 16.1 | p95 | 11 | 15.8 | 120.5 | 19/19 | 2 / 2 | 8775 MB | 4.02-4.0 | MISS: p95 15.8 ms > 11 ms |
| plain-1000 | 1001 | letter@middle | 19 | 17.7 / 21.6 / 22.2 | p95 | 11 | 17.9 | 100.3 | 19/19 | 3 / 3 | 8775 MB | 4.0-3.85 | MISS: p95 21.6 ms > 11 ms |
| plain-1000 | 1001 | letter@end | 19 | 18.3 / 19.0 / 19.1 | p95 | 11 | 19.0 | 71.1 | 19/19 | 2 / 2 | 8775 MB | 3.85-3.86 | MISS: p95 19.0 ms > 11 ms |
| plain-1000 | 1001 | sentence@middle | 19 | 25.8 / 46.1 / 53.2 | p95 | 11 | 33.0 | 11181.0 | 0/19 | 1503 / 1505 | 8775 MB | 3.86-10.31 | MISS: p95 46.1 ms > 11 ms |
| plain-1000 | 1001 | newline@middle | 19 | 19.0 / 36.4 / 142.1 | p95 | 11 | 34.1 | 3715.8 | 0/19 | 502 / 502 | 8775 MB | 10.31-12.58 | MISS: p95 36.4 ms > 11 ms |
| plain-1000 | 1001 | split@middle | 19 | 25.1 / 33.2 / 40.8 | p95 | 11 | 28.6 | 9730.8 | 0/19 | 1503 / 1505 | 8775 MB | 12.58-6.32 | MISS: p95 33.2 ms > 11 ms |
| plain-1000 | 1001 | preamble | 5 | 198.6 / 292.1 / 292.1 | max | 400 | 292.1 | 3963.6 | 0/5 | 1001 / 1001 | 8775 MB | 6.32-6.63 | PASS |
| plain-1000 | 1001 | reopen: host's share, cold (spawn -> page 1) | 5 | 562.8 / 583.6 / 583.6 | max | (100, app) | - | - | - | - | 8775 MB | 6.13-6.36 | report-only: max 583.6 ms > 100 ms |
| plain-1000 | 1001 | reopen: host already listening (not counted, decision 8) | 5 | 30.3 / 33.4 / 33.4 | max | (100, app) | - | - | - | - | 8775 MB | 6.13-6.36 | report-only |
| full-1000 | 1002 | letter@start | 19 | 22.6 / 25.5 / 29.6 | p95 | 11 | 25.5 | 224.0 | 19/19 | 2 / 2 | 8874 MB | 6.18-7.93 | MISS: p95 25.5 ms > 11 ms |
| full-1000 | 1002 | letter@middle | 19 | 46.5 / 69.9 / 80.9 | p95 | 11 | 69.9 | 803.5 | 19/19 | 33 / 33 | 8874 MB | 7.93-7.75 | MISS: p95 69.9 ms > 11 ms |
| full-1000 | 1002 | letter@end | 19 | 79.3 / 121.5 / 164.8 | p95 | 11 | 106.7 | 428.3 | 19/19 | 6 / 6 | 8874 MB | 7.75-9.1 | MISS: p95 121.5 ms > 11 ms |
| full-1000 | 1002 | sentence@middle | 19 | 49.2 / 82.9 / 103.6 | p95 | 11 | 82.9 | 886.1 | 19/19 | 33 / 33 | 8874 MB | 9.1-9.4 | MISS: p95 82.9 ms > 11 ms |
| full-1000 | 1002 | newline@middle | 19 | 50.5 / 69.0 / 94.6 | p95 | 11 | 69.0 | 7570.9 | 0/19 | 501 / 501 | 8874 MB | 9.4-6.93 | MISS: p95 69.0 ms > 11 ms |
| full-1000 | 1002 | split@middle | 19 | 52.6 / 59.3 / 61.0 | p95 | 11 | 59.3 | 7613.4 | 0/19 | 501 / 501 | 8874 MB | 6.93-4.48 | MISS: p95 59.3 ms > 11 ms |
| full-1000 | 1002 | preamble | 5 | 4266.4 / 4744.5 / 4744.5 | max | 400 | 4744.5 | 20157.2 | 0/5 | 1002 / 1002 | 8874 MB | 4.48-12.88 | MISS: max 4744.5 ms > 400 ms |
| full-1000 | 1002 | reopen: host's share, cold (spawn -> page 1) | 5 | 692.8 / 707.2 / 707.2 | max | (100, app) | - | - | - | - | 8874 MB | 7.93-10.98 | report-only: max 707.2 ms > 100 ms |
| full-1000 | 1002 | reopen: host already listening (not counted, decision 8) | 5 | 157.2 / 159.1 / 159.1 | max | (100, app) | - | - | - | - | 8874 MB | 7.93-10.98 | report-only: max 159.1 ms > 100 ms |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1). The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.

T7: 48 MISSES (8 of 56 gated rows pass; NON-REFERENCE (power state not recorded; load1 up to 12.9 (> 5.0)) run)
  plain-10 letter@start: p95 18.5 ms > 11 ms
  plain-10 sentence@middle: p95 11.3 ms > 11 ms
  plain-10 newline@middle: p95 11.9 ms > 11 ms
  full-10 letter@start: p95 35.4 ms > 11 ms
  full-10 letter@middle: p95 22.3 ms > 11 ms
  full-10 letter@end: p95 37.7 ms > 11 ms
  full-10 sentence@middle: p95 23.0 ms > 11 ms
  full-10 newline@middle: p95 23.2 ms > 11 ms
  full-10 split@middle: p95 22.9 ms > 11 ms
  full-10 preamble: max 428.1 ms > 400 ms
  plain-100 letter@start: p95 12.4 ms > 11 ms
  plain-100 letter@middle: p95 14.4 ms > 11 ms
  plain-100 letter@end: p95 22.1 ms > 11 ms
  plain-100 sentence@middle: p95 14.8 ms > 11 ms
  plain-100 newline@middle: p95 15.6 ms > 11 ms
  plain-100 split@middle: p95 16.0 ms > 11 ms
  full-100 letter@start: p95 22.6 ms > 11 ms
  full-100 letter@middle: p95 19.1 ms > 11 ms
  full-100 letter@end: p95 40.1 ms > 11 ms
  full-100 sentence@middle: p95 20.5 ms > 11 ms
  full-100 newline@middle: p95 20.8 ms > 11 ms
  full-100 split@middle: p95 21.2 ms > 11 ms
  full-100 preamble: max 445.5 ms > 400 ms
  plain-300 letter@start: p95 12.2 ms > 11 ms
  plain-300 letter@middle: p95 11.8 ms > 11 ms
  plain-300 letter@end: p95 23.2 ms > 11 ms
  plain-300 newline@middle: p95 11.4 ms > 11 ms
  plain-300 split@middle: p95 13.9 ms > 11 ms
  full-300 letter@start: p95 21.6 ms > 11 ms
  full-300 letter@middle: p95 89.3 ms > 11 ms
  full-300 letter@end: p95 61.6 ms > 11 ms
  full-300 sentence@middle: p95 42.1 ms > 11 ms
  full-300 newline@middle: p95 41.7 ms > 11 ms
  full-300 split@middle: p95 40.1 ms > 11 ms
  full-300 preamble: max 533.2 ms > 400 ms
  plain-1000 letter@start: p95 15.8 ms > 11 ms
  plain-1000 letter@middle: p95 21.6 ms > 11 ms
  plain-1000 letter@end: p95 19.0 ms > 11 ms
  plain-1000 sentence@middle: p95 46.1 ms > 11 ms
  plain-1000 newline@middle: p95 36.4 ms > 11 ms
  plain-1000 split@middle: p95 33.2 ms > 11 ms
  full-1000 letter@start: p95 25.5 ms > 11 ms
  full-1000 letter@middle: p95 69.9 ms > 11 ms
  full-1000 letter@end: p95 121.5 ms > 11 ms
  full-1000 sentence@middle: p95 82.9 ms > 11 ms
  full-1000 newline@middle: p95 69.0 ms > 11 ms
  full-1000 split@middle: p95 59.3 ms > 11 ms
  full-1000 preamble: max 4744.5 ms > 400 ms
