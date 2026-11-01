**NON-REFERENCE run**: power state not recorded; load1 up to 147.3 (> 5.0)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| plain-10 | 10 | letter@start | 19 | 18.8 / 36.4 / 51.8 | p95 | 11 | 36.4 | 197.5 | 19/19 | 9 / 9 | 286 MB | 48.14-44.29 | MISS: p95 36.4 ms > 11 ms |
| plain-10 | 10 | letter@middle | 19 | 10.6 / 15.4 / 22.3 | p95 | 11 | 12.1 | 124.9 | 19/19 | 6 / 6 | 286 MB | 44.29-43.63 | MISS: p95 15.4 ms > 11 ms |
| plain-10 | 10 | letter@end | 19 | 7.2 / 11.8 / 15.6 | p95 | 11 | 11.8 | 50.0 | 0/19 | 1 / 1 | 286 MB | 43.63-43.82 | MISS: p95 11.8 ms > 11 ms |
| plain-10 | 10 | sentence@middle | 19 | 10.9 / 17.2 / 21.9 | p95 | 11 | 13.1 | 70.3 | 0/19 | 6 / 6 | 286 MB | 43.82-41.44 | MISS: p95 17.2 ms > 11 ms |
| plain-10 | 10 | newline@middle | 19 | 10.3 / 15.3 / 20.4 | p95 | 11 | 11.6 | 58.6 | 0/19 | 6 / 6 | 286 MB | 41.44-39.1 | MISS: p95 15.3 ms > 11 ms |
| plain-10 | 10 | split@middle | 19 | 11.1 / 20.2 / 20.2 | p95 | 11 | 12.4 | 89.8 | 0/19 | 6 / 6 | 286 MB | 39.1-37.89 | MISS: p95 20.2 ms > 11 ms |
| plain-10 | 10 | preamble | 5 | 136.8 / 152.4 / 152.4 | max | 400 | 152.4 | 199.5 | 0/5 | 10 / 10 | 286 MB | 37.89-37.5 | PASS |
| plain-10 | 10 | reopen: host's share, cold (spawn -> page 1) | 5 | 622.0 / 789.0 / 789.0 | max | (100, app) | - | - | - | - | 286 MB | 35.45-37.5 | report-only: max 789.0 ms > 100 ms |
| plain-10 | 10 | reopen: host already listening (not counted, decision 8) | 5 | 36.7 / 40.3 / 40.3 | max | (100, app) | - | - | - | - | 286 MB | 35.45-37.5 | report-only |
| full-10 | 11 | letter@start | 19 | 37.0 / 115.7 / 159.0 | p95 | 11 | 115.7 | 296.8 | 0/19 | 11 / 11 | 481 MB | 35.45-31.45 | MISS: p95 115.7 ms > 11 ms |
| full-10 | 11 | letter@middle | 19 | 20.1 / 26.0 / 26.2 | p95 | 11 | 15.6 | 160.7 | 19/19 | 4 / 4 | 481 MB | 31.45-31.9 | MISS: p95 26.0 ms > 11 ms |
| full-10 | 11 | letter@end | 19 | 41.6 / 77.4 / 109.2 | p95 | 11 | 58.3 | 152.5 | 0/19 | 3 / 3 | 481 MB | 31.9-32.25 | MISS: p95 77.4 ms > 11 ms |
| full-10 | 11 | sentence@middle | 19 | 18.8 / 54.9 / 60.0 | p95 | 11 | 24.8 | 162.9 | 0/19 | 7 / 7 | 481 MB | 32.25-30.0 | MISS: p95 54.9 ms > 11 ms |
| full-10 | 11 | newline@middle | 19 | 17.3 / 20.1 / 20.5 | p95 | 11 | 9.8 | 82.0 | 0/19 | 7 / 7 | 481 MB | 30.0-28.16 | MISS: p95 20.1 ms > 11 ms |
| full-10 | 11 | split@middle | 19 | 18.0 / 41.0 / 47.3 | p95 | 11 | 23.2 | 232.5 | 0/19 | 7 / 7 | 481 MB | 28.16-27.93 | MISS: p95 41.0 ms > 11 ms |
| full-10 | 11 | preamble | 5 | 429.0 / 515.5 / 515.5 | max | 400 | 515.5 | 609.4 | 0/5 | 11 / 11 | 481 MB | 27.93-26.65 | MISS: max 515.5 ms > 400 ms |
| full-10 | 11 | reopen: host's share, cold (spawn -> page 1) | 5 | 671.1 / 686.0 / 686.0 | max | (100, app) | - | - | - | - | 481 MB | 23.93-25.32 | report-only: max 686.0 ms > 100 ms |
| full-10 | 11 | reopen: host already listening (not counted, decision 8) | 5 | 115.2 / 119.0 / 119.0 | max | (100, app) | - | - | - | - | 481 MB | 23.93-25.32 | report-only: max 119.0 ms > 100 ms |
| plain-100 | 100 | letter@start | 19 | 7.4 / 22.7 / 49.7 | p95 | 11 | 22.7 | 86.1 | 19/19 | 2 / 2 | 937 MB | 23.93-32.44 | MISS: p95 22.7 ms > 11 ms |
| plain-100 | 100 | letter@middle | 19 | 12.7 / 14.5 / 14.9 | p95 | 11 | 14.5 | 265.3 | 19/19 | 33 / 33 | 937 MB | 32.44-28.76 | MISS: p95 14.5 ms > 11 ms |
| plain-100 | 100 | letter@end | 19 | 10.1 / 10.7 / 31.9 | p95 | 11 | 6.9 | 39.0 | 0/19 | 3 / 3 | 937 MB | 28.76-27.17 | PASS |
| plain-100 | 100 | sentence@middle | 19 | 12.3 / 22.1 / 32.5 | p95 | 11 | 22.1 | 260.6 | 0/19 | 50 / 50 | 937 MB | 27.17-24.51 | MISS: p95 22.1 ms > 11 ms |
| plain-100 | 100 | newline@middle | 19 | 12.4 / 14.1 / 17.0 | p95 | 11 | 14.1 | 241.7 | 0/19 | 50 / 50 | 937 MB | 24.51-22.56 | MISS: p95 14.1 ms > 11 ms |
| plain-100 | 100 | split@middle | 19 | 12.6 / 28.3 / 31.7 | p95 | 11 | 28.3 | 320.3 | 0/19 | 50 / 50 | 937 MB | 22.56-22.16 | MISS: p95 28.3 ms > 11 ms |
| plain-100 | 100 | preamble | 5 | 100.9 / 119.2 / 119.2 | max | 400 | 119.2 | 544.6 | 0/5 | 100 / 100 | 937 MB | 22.16-20.95 | PASS |
| plain-100 | 100 | reopen: host's share, cold (spawn -> page 1) | 5 | 551.6 / 552.5 / 552.5 | max | (100, app) | - | - | - | - | 937 MB | 19.83-19.83 | report-only: max 552.5 ms > 100 ms |
| plain-100 | 100 | reopen: host already listening (not counted, decision 8) | 5 | 27.8 / 32.9 / 32.9 | max | (100, app) | - | - | - | - | 937 MB | 19.83-19.83 | report-only |
| full-100 | 101 | letter@start | 19 | 15.5 / 53.1 / 122.7 | p95 | 11 | 53.1 | 2519.7 | 0/19 | 99 / 99 | 2434 MB | 18.8-40.53 | MISS: p95 53.1 ms > 11 ms |
| full-100 | 101 | letter@middle | 19 | 17.0 / 43.1 / 64.8 | p95 | 11 | 43.1 | 219.8 | 19/19 | 2 / 2 | 2434 MB | 40.53-45.27 | MISS: p95 43.1 ms > 11 ms |
| full-100 | 101 | letter@end | 19 | 44.2 / 54.2 / 64.1 | p95 | 11 | 43.2 | 148.6 | 0/19 | 4 / 4 | 2434 MB | 45.27-45.16 | MISS: p95 54.2 ms > 11 ms |
| full-100 | 101 | sentence@middle | 19 | 14.6 / 20.0 / 62.3 | p95 | 11 | 20.0 | 534.8 | 19/19 | 33 / 33 | 2434 MB | 45.16-37.13 | MISS: p95 20.0 ms > 11 ms |
| full-100 | 101 | newline@middle | 19 | 14.3 / 23.5 / 94.4 | p95 | 11 | 23.5 | 711.0 | 0/19 | 51 / 51 | 2434 MB | 37.13-29.22 | MISS: p95 23.5 ms > 11 ms |
| full-100 | 101 | split@middle | 19 | 14.7 / 21.6 / 44.9 | p95 | 11 | 21.6 | 671.3 | 0/19 | 51 / 51 | 2434 MB | 29.22-24.82 | MISS: p95 21.6 ms > 11 ms |
| full-100 | 101 | preamble | 5 | 761.1 / 1075.8 / 1075.8 | max | 400 | 1075.8 | 4970.0 | 0/5 | 101 / 101 | 2434 MB | 24.82-27.12 | MISS: max 1075.8 ms > 400 ms |
| full-100 | 101 | reopen: host's share, cold (spawn -> page 1) | 5 | 656.2 / 822.4 / 822.4 | max | (100, app) | - | - | - | - | 2434 MB | 23.44-26.07 | report-only: max 822.4 ms > 100 ms |
| full-100 | 101 | reopen: host already listening (not counted, decision 8) | 5 | 104.8 / 120.2 / 120.2 | max | (100, app) | - | - | - | - | 2434 MB | 23.44-26.07 | report-only: max 120.2 ms > 100 ms |
| plain-300 | 301 | letter@start | 19 | 7.7 / 8.4 / 9.1 | p95 | 11 | 8.4 | 130.4 | 19/19 | 5 / 5 | 2552 MB | 23.44-20.72 | PASS |
| plain-300 | 301 | letter@middle | 19 | 10.1 / 34.4 / 36.3 | p95 | 11 | 34.4 | 417.8 | 19/19 | 9 / 9 | 2552 MB | 20.72-24.73 | MISS: p95 34.4 ms > 11 ms |
| plain-300 | 301 | letter@end | 19 | 33.1 / 95.1 / 112.2 | p95 | 11 | 95.1 | 305.2 | 0/19 | 7 / 7 | 2552 MB | 24.73-35.56 | MISS: p95 95.1 ms > 11 ms |
| plain-300 | 301 | sentence@middle | 19 | 10.0 / 24.9 / 36.3 | p95 | 11 | 24.9 | 946.7 | 19/19 | 65 / 65 | 2552 MB | 35.56-33.71 | MISS: p95 24.9 ms > 11 ms |
| plain-300 | 301 | newline@middle | 19 | 7.7 / 8.3 / 8.3 | p95 | 11 | 8.3 | 646.2 | 0/19 | 151 / 151 | 2552 MB | 33.71-27.84 | PASS |
| plain-300 | 301 | split@middle | 19 | 15.7 / 47.5 / 53.3 | p95 | 11 | 47.5 | 2549.0 | 0/19 | 151 / 151 | 2552 MB | 27.84-60.89 | MISS: p95 47.5 ms > 11 ms |
| plain-300 | 301 | preamble | 5 | 134.6 / 219.7 / 219.7 | max | 400 | 219.7 | 2934.2 | 0/5 | 301 / 301 | 2552 MB | 60.89-52.03 | PASS |
| plain-300 | 301 | reopen: host's share, cold (spawn -> page 1) | 5 | 752.7 / 786.7 / 786.7 | max | (100, app) | - | - | - | - | 2552 MB | 44.38-52.35 | report-only: max 786.7 ms > 100 ms |
| plain-300 | 301 | reopen: host already listening (not counted, decision 8) | 5 | 42.8 / 62.5 / 62.5 | max | (100, app) | - | - | - | - | 2552 MB | 44.38-52.35 | report-only |
| full-300 | 299 | letter@start | 19 | 13.7 / 18.7 / 19.4 | p95 | 11 | 18.7 | 163.3 | 19/19 | 3 / 3 | 5553 MB | 44.38-36.9 | MISS: p95 18.7 ms > 11 ms |
| full-300 | 299 | letter@middle | 19 | 35.8 / 69.4 / 84.5 | p95 | 11 | 69.4 | 225.2 | 19/19 | 2 / 2 | 5553 MB | 36.9-32.7 | MISS: p95 69.4 ms > 11 ms |
| full-300 | 299 | letter@end | 19 | 81.1 / 766.5 / 859.1 | p95 | 11 | 682.7 | 1491.7 | 19/19 | 3 / 3 | 5553 MB | 32.7-124.36 | MISS: p95 766.5 ms > 11 ms |
| full-300 | 299 | sentence@middle | 19 | 36.4 / 76.2 / 165.8 | p95 | 11 | 76.2 | 1484.3 | 19/19 | 33 / 33 | 5553 MB | 124.36-147.28 | MISS: p95 76.2 ms > 11 ms |
| full-300 | 299 | newline@middle | 19 | 36.6 / 42.8 / 119.2 | p95 | 11 | 42.8 | 1892.9 | 0/19 | 150 / 150 | 5553 MB | 147.28-86.61 | MISS: p95 42.8 ms > 11 ms |
| full-300 | 299 | split@middle | 19 | 39.3 / 45.6 / 88.4 | p95 | 11 | 45.6 | 1866.2 | 0/19 | 150 / 150 | 5553 MB | 86.61-54.86 | MISS: p95 45.6 ms > 11 ms |
| full-300 | 299 | preamble | 5 | 498.1 / 540.8 / 540.8 | max | 400 | 540.8 | 3728.4 | 0/5 | 299 / 299 | 5553 MB | 54.86-45.73 | MISS: max 540.8 ms > 400 ms |
| full-300 | 299 | reopen: host's share, cold (spawn -> page 1) | 5 | 1141.5 / 1505.3 / 1505.3 | max | (100, app) | - | - | - | - | 5553 MB | 42.61-57.95 | report-only: max 1505.3 ms > 100 ms |
| full-300 | 299 | reopen: host already listening (not counted, decision 8) | 5 | 284.3 / 317.8 / 317.8 | max | (100, app) | - | - | - | - | 5553 MB | 42.61-57.95 | report-only: max 317.8 ms > 100 ms |
| plain-1000 | 1001 | letter@start | 19 | 13.7 / 22.1 / 24.0 | p95 | 11 | 22.1 | 332.9 | 19/19 | 2 / 2 | 6842 MB | 56.27-96.55 | MISS: p95 22.1 ms > 11 ms |
| plain-1000 | 1001 | letter@middle | 19 | 22.4 / 56.5 / 56.5 | p95 | 11 | 27.2 | 290.2 | 19/19 | 3 / 3 | 6842 MB | 96.55-85.5 | MISS: p95 56.5 ms > 11 ms |
| plain-1000 | 1001 | letter@end | 19 | 22.1 / 58.9 / 109.1 | p95 | 11 | 58.9 | 220.5 | 19/19 | 2 / 2 | 6842 MB | 85.5-79.24 | MISS: p95 58.9 ms > 11 ms |
| plain-1000 | 1001 | sentence@middle | 19 | 28.9 / 31.7 / 36.4 | p95 | 11 | 26.0 | 11050.7 | 0/19 | 1503 / 1505 | 6842 MB | 79.24-10.34 | MISS: p95 31.7 ms > 11 ms |
| plain-1000 | 1001 | newline@middle | 19 | 14.6 / 21.2 / 22.7 | p95 | 11 | 16.2 | 2798.3 | 0/19 | 502 / 502 | 6842 MB | 10.34-8.39 | MISS: p95 21.2 ms > 11 ms |
| plain-1000 | 1001 | split@middle | 19 | 30.7 / 46.6 / 94.5 | p95 | 11 | 41.1 | 20642.7 | 0/19 | 1503 / 1505 | 6842 MB | 8.39-70.99 | MISS: p95 46.6 ms > 11 ms |
| plain-1000 | 1001 | preamble | 5 | 317.5 / 1791.6 / 1791.6 | max | 400 | 1791.6 | 6746.7 | 0/5 | 1001 / 1001 | 6842 MB | 70.99-43.83 | MISS: max 1791.6 ms > 400 ms |
| plain-1000 | 1001 | reopen: host's share, cold (spawn -> page 1) | 5 | 573.5 / 648.9 / 648.9 | max | (100, app) | - | - | - | - | 6842 MB | 29.18-35.38 | report-only: max 648.9 ms > 100 ms |
| plain-1000 | 1001 | reopen: host already listening (not counted, decision 8) | 5 | 31.1 / 32.5 / 32.5 | max | (100, app) | - | - | - | - | 6842 MB | 29.18-35.38 | report-only |
| full-1000 | 1002 | letter@start | 19 | 22.5 / 25.0 / 26.9 | p95 | 11 | 25.0 | 229.3 | 19/19 | 2 / 2 | 8608 MB | 27.57-15.41 | MISS: p95 25.0 ms > 11 ms |
| full-1000 | 1002 | letter@middle | 19 | 40.7 / 41.7 / 43.0 | p95 | 11 | 41.7 | 644.1 | 19/19 | 33 / 33 | 8608 MB | 15.41-12.9 | MISS: p95 41.7 ms > 11 ms |
| full-1000 | 1002 | letter@end | 19 | 64.0 / 85.2 / 106.7 | p95 | 11 | 77.7 | 288.8 | 19/19 | 6 / 6 | 8608 MB | 12.9-11.38 | MISS: p95 85.2 ms > 11 ms |
| full-1000 | 1002 | sentence@middle | 19 | 40.1 / 41.7 / 150.3 | p95 | 11 | 41.7 | 676.8 | 19/19 | 33 / 33 | 8608 MB | 11.38-9.45 | MISS: p95 41.7 ms > 11 ms |
| full-1000 | 1002 | newline@middle | 19 | 53.9 / 75.1 / 203.9 | p95 | 11 | 75.1 | 8138.8 | 0/19 | 501 / 501 | 8608 MB | 9.45-6.75 | MISS: p95 75.1 ms > 11 ms |
| full-1000 | 1002 | split@middle | 19 | 51.8 / 55.3 / 58.3 | p95 | 11 | 55.3 | 7415.0 | 0/19 | 501 / 501 | 8608 MB | 6.75-8.94 | MISS: p95 55.3 ms > 11 ms |
| full-1000 | 1002 | preamble | 5 | 4464.2 / 4708.9 / 4708.9 | max | 400 | 4708.9 | 20081.4 | 0/5 | 1002 / 1002 | 8608 MB | 8.94-8.68 | MISS: max 4708.9 ms > 400 ms |
| full-1000 | 1002 | reopen: host's share, cold (spawn -> page 1) | 5 | 678.0 / 715.3 / 715.3 | max | (100, app) | - | - | - | - | 8608 MB | 5.54-6.88 | report-only: max 715.3 ms > 100 ms |
| full-1000 | 1002 | reopen: host already listening (not counted, decision 8) | 5 | 151.6 / 175.1 / 175.1 | max | (100, app) | - | - | - | - | 8608 MB | 5.54-6.88 | report-only: max 175.1 ms > 100 ms |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1). The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.

T7: 50 MISSES (6 of 56 gated rows pass; NON-REFERENCE (power state not recorded; load1 up to 147.3 (> 5.0)) run)
  plain-10 letter@start: p95 36.4 ms > 11 ms
  plain-10 letter@middle: p95 15.4 ms > 11 ms
  plain-10 letter@end: p95 11.8 ms > 11 ms
  plain-10 sentence@middle: p95 17.2 ms > 11 ms
  plain-10 newline@middle: p95 15.3 ms > 11 ms
  plain-10 split@middle: p95 20.2 ms > 11 ms
  full-10 letter@start: p95 115.7 ms > 11 ms
  full-10 letter@middle: p95 26.0 ms > 11 ms
  full-10 letter@end: p95 77.4 ms > 11 ms
  full-10 sentence@middle: p95 54.9 ms > 11 ms
  full-10 newline@middle: p95 20.1 ms > 11 ms
  full-10 split@middle: p95 41.0 ms > 11 ms
  full-10 preamble: max 515.5 ms > 400 ms
  plain-100 letter@start: p95 22.7 ms > 11 ms
  plain-100 letter@middle: p95 14.5 ms > 11 ms
  plain-100 sentence@middle: p95 22.1 ms > 11 ms
  plain-100 newline@middle: p95 14.1 ms > 11 ms
  plain-100 split@middle: p95 28.3 ms > 11 ms
  full-100 letter@start: p95 53.1 ms > 11 ms
  full-100 letter@middle: p95 43.1 ms > 11 ms
  full-100 letter@end: p95 54.2 ms > 11 ms
  full-100 sentence@middle: p95 20.0 ms > 11 ms
  full-100 newline@middle: p95 23.5 ms > 11 ms
  full-100 split@middle: p95 21.6 ms > 11 ms
  full-100 preamble: max 1075.8 ms > 400 ms
  plain-300 letter@middle: p95 34.4 ms > 11 ms
  plain-300 letter@end: p95 95.1 ms > 11 ms
  plain-300 sentence@middle: p95 24.9 ms > 11 ms
  plain-300 split@middle: p95 47.5 ms > 11 ms
  full-300 letter@start: p95 18.7 ms > 11 ms
  full-300 letter@middle: p95 69.4 ms > 11 ms
  full-300 letter@end: p95 766.5 ms > 11 ms
  full-300 sentence@middle: p95 76.2 ms > 11 ms
  full-300 newline@middle: p95 42.8 ms > 11 ms
  full-300 split@middle: p95 45.6 ms > 11 ms
  full-300 preamble: max 540.8 ms > 400 ms
  plain-1000 letter@start: p95 22.1 ms > 11 ms
  plain-1000 letter@middle: p95 56.5 ms > 11 ms
  plain-1000 letter@end: p95 58.9 ms > 11 ms
  plain-1000 sentence@middle: p95 31.7 ms > 11 ms
  plain-1000 newline@middle: p95 21.2 ms > 11 ms
  plain-1000 split@middle: p95 46.6 ms > 11 ms
  plain-1000 preamble: max 1791.6 ms > 400 ms
  full-1000 letter@start: p95 25.0 ms > 11 ms
  full-1000 letter@middle: p95 41.7 ms > 11 ms
  full-1000 letter@end: p95 85.2 ms > 11 ms
  full-1000 sentence@middle: p95 41.7 ms > 11 ms
  full-1000 newline@middle: p95 75.1 ms > 11 ms
  full-1000 split@middle: p95 55.3 ms > 11 ms
  full-1000 preamble: max 4708.9 ms > 400 ms
