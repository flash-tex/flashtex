# T7 reference run, 2026-10-21, Kabirs-MacBook-Pro

P4 gate (DESIGN.md §12; Q1 decided 2026-10-05): `scripts/t7-reference.sh`, the full T7 run on an
idle, plugged-in Apple-Silicon Mac with Low Power Mode off.

- **Verdict:** T7: 2 MISSES (118 of 120 gated rows pass; NON-REFERENCE (on battery at start (42 %); on battery at end (34 %)) run)
- **Exit:** 1: a gated target missed (the table's MISS rows)
- **Commit:** `5439b97ea` (Merge pull request #1860 from flash-tex/agent/kabir-claude/t7-last12)
- **Machine:** Mac17,9 Apple M5 Pro 26.6.2
- **Checks failed (--force):** not on mains power: Now drawing from 'Battery Power'
- **Command:** `scripts/t7-reference.sh --force --max-load 100 --wait 0`, which ran `t7.py --engine ref --out /tmp/flashtex-t7-reference/t7-reference-run --max-load 100 --wait-load 100 --wait-max 0 --build`

Files: `table.md` (the rows), `summary.json.gz` (`t7.py --check` re-evaluates it after
gunzip), `raw.tgz` (dl3-keys' lines), `environment.txt`, `t7-output.txt`.

**NON-REFERENCE run**: on battery at start (42 %); on battery at end (34 %)

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | off-CPU p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| plain-10 | 10 | letter@start | 19 | 7.3 / 9.3 / 10.1 | p95 | 11 | 9.3 | 25.1 | 2.1 | 19/19 | 2 / 2 | 190 MB | 2.16-2.14 | PASS |
| plain-10 | 10 | letter@middle | 19 | 4.2 / 5.7 / 5.7 | p95 | 11 | 4.2 | 18.3 | 0.7 | 19/19 | 3 / 3 | 190 MB | 2.14-1.97 | PASS |
| plain-10 | 10 | letter@end | 19 | 3.0 / 4.2 / 4.7 | p95 | 11 | 4.2 | 10.1 | 0.8 | 0/19 | 1 / 1 | 190 MB | 1.97-1.81 | PASS |
| plain-10 | 10 | sentence@middle | 19 | 4.3 / 5.5 / 5.7 | p95 | 11 | 3.9 | 22.5 | 0.8 | 0/19 | 6 / 6 | 190 MB | 1.81-1.69 | PASS |
| plain-10 | 10 | newline@middle | 19 | 4.3 / 5.2 / 5.2 | p95 | 11 | 3.6 | 17.6 | 0.1 | 19/19 | 3 / 3 | 190 MB | 1.69-1.55 | PASS |
| plain-10 | 10 | split@middle | 19 | 4.2 / 5.5 / 5.6 | p95 | 11 | 3.8 | 21.8 | 0.8 | 0/19 | 6 / 6 | 190 MB | 1.55-1.59 | PASS |
| plain-10 | 10 | typing@50ms | 39 | 3.7 / 4.3 / 7.2 | p95 | 11 | - | - | 0.2 | - | - | 190 MB | 1.59-1.62 | PASS |
| plain-10 | 10 | typing@60ms | 39 | 3.3 / 3.6 / 3.8 | p95 | 11 | - | - | 0.1 | - | - | 190 MB | 1.62-1.62 | PASS |
| plain-10 | 10 | typing@80ms | 39 | 4.0 / 5.2 / 7.2 | p95 | 11 | - | - | 0.4 | - | - | 190 MB | 1.62-1.97 | PASS |
| plain-10 | 10 | typing@100ms | 39 | 4.0 / 4.3 / 4.7 | p95 | 11 | - | - | 0.7 | - | - | 190 MB | 1.97-1.81 | PASS |
| plain-10 | 10 | typing@150ms | 39 | 4.3 / 5.0 / 5.4 | p95 | 11 | - | - | 0.6 | - | - | 190 MB | 1.81-1.67 | PASS |
| plain-10 | 10 | preamble | 13 | 36.3 / 48.9 / 54.3 | p95 | 400 | 48.9 | 67.9 | 1.7 | 0/13 | 10 / 10 | 190 MB | 1.67-1.53 | PASS |
| plain-10 | 10 | reopen: host's share, cold (spawn -> page 1) | 12 | 55.5 / 56.6 / 57.1 | p95 | (100, app) | - | - | - | - | - | 190 MB | 1.49-1.53 | report-only |
| plain-10 | 10 | reopen: host already listening (not counted, decision 8) | 12 | 16.7 / 16.8 / 16.9 | p95 | (100, app) | - | - | - | - | - | 190 MB | 1.49-1.53 | report-only |
| full-10 | 11 | letter@middle | 19 | 8.0 / 9.1 / 9.1 | p95 | 11 | 5.4 | 29.2 | 0.9 | 19/19 | 3 / 3 | 237 MB | 1.45-1.74 | PASS |
| full-10 | 11 | letter@end | 19 | 7.4 / 9.0 / 10.0 | p95 | 11 | 5.2 | 25.2 | 0.8 | 19/19 | 3 / 3 | 237 MB | 1.74-1.76 | PASS |
| full-10 | 11 | sentence@middle | 19 | 8.1 / 9.0 / 9.1 | p95 | 11 | 5.3 | 41.3 | 0.8 | 0/19 | 7 / 7 | 237 MB | 1.76-1.78 | PASS |
| full-10 | 11 | newline@middle | 19 | 7.8 / 8.7 / 12.5 | p95 | 11 | 5.0 | 35.4 | 0.2 | 19/19 | 3 / 3 | 237 MB | 1.78-1.88 | PASS |
| full-10 | 11 | split@middle | 19 | 7.8 / 8.7 / 8.9 | p95 | 11 | 4.6 | 35.1 | 0.6 | 19/19 | 3 / 3 | 237 MB | 1.88-1.73 | PASS |
| full-10 | 11 | letter@start | 19 | 11.5 / 12.2 / 12.4 | p95 | 11 | 12.2 | 35.8 | 2.3 | 19/19 | 2 / 2 | 237 MB | 1.73-1.7 | MISS: p95 12.2 ms > 11 ms |
| full-10 | 11 | typing@50ms | 39 | 6.6 / 7.7 / 8.2 | p95 | 11 | - | - | 0.2 | - | - | 237 MB | 1.7-1.7 | PASS |
| full-10 | 11 | typing@60ms | 39 | 7.1 / 8.2 / 8.5 | p95 | 11 | - | - | 0.2 | - | - | 237 MB | 1.7-1.8 | PASS |
| full-10 | 11 | typing@80ms | 39 | 6.6 / 7.3 / 7.7 | p95 | 11 | - | - | 0.5 | - | - | 237 MB | 1.8-1.8 | PASS |
| full-10 | 11 | typing@100ms | 39 | 7.0 / 7.4 / 11.8 | p95 | 11 | - | - | 0.5 | - | - | 237 MB | 1.8-2.14 | PASS |
| full-10 | 11 | typing@150ms | 39 | 7.7 / 9.4 / 11.9 | p95 | 11 | - | - | 0.8 | - | - | 237 MB | 2.14-1.96 | PASS |
| full-10 | 11 | preamble | 13 | 161.9 / 166.8 / 175.5 | p95 | 400 | 166.8 | 201.2 | 3.2 | 0/13 | 11 / 11 | 237 MB | 1.96-2.05 | PASS |
| full-10 | 11 | reopen: host's share, cold (spawn -> page 1) | 12 | 88.5 / 92.4 / 96.7 | p95 | (100, app) | - | - | - | - | - | 237 MB | 1.96-2.05 | report-only |
| full-10 | 11 | reopen: host already listening (not counted, decision 8) | 12 | 50.0 / 52.6 / 56.0 | p95 | (100, app) | - | - | - | - | - | 237 MB | 1.96-2.05 | report-only |
| plain-100 | 100 | letter@end | 19 | 5.0 / 5.6 / 5.6 | p95 | 11 | 3.9 | 17.6 | 0.7 | 19/19 | 3 / 3 | 194 MB | 1.89-1.98 | PASS |
| plain-100 | 100 | sentence@middle | 19 | 3.9 / 4.4 / 4.6 | p95 | 11 | 4.4 | 78.2 | 0.8 | 0/19 | 50 / 50 | 194 MB | 1.98-1.9 | PASS |
| plain-100 | 100 | newline@middle | 19 | 3.2 / 4.3 / 4.4 | p95 | 11 | 4.3 | 15.3 | 0.1 | 19/19 | 2 / 2 | 194 MB | 1.9-2.07 | PASS |
| plain-100 | 100 | split@middle | 19 | 4.5 / 4.9 / 4.9 | p95 | 11 | 4.9 | 18.6 | 0.8 | 19/19 | 2 / 2 | 194 MB | 2.07-1.91 | PASS |
| plain-100 | 100 | letter@start | 19 | 3.5 / 5.0 / 5.2 | p95 | 11 | 5.0 | 19.6 | 0.8 | 19/19 | 2 / 2 | 194 MB | 1.91-1.75 | PASS |
| plain-100 | 100 | letter@middle | 19 | 3.4 / 5.0 / 11.1 | p95 | 11 | 5.0 | 18.6 | 0.8 | 19/19 | 2 / 2 | 194 MB | 1.75-1.88 | PASS |
| plain-100 | 100 | typing@50ms | 39 | 2.9 / 5.1 / 10.4 | p95 | 11 | - | - | 1.2 | - | - | 194 MB | 1.88-1.88 | PASS |
| plain-100 | 100 | typing@60ms | 39 | 2.7 / 3.2 / 3.5 | p95 | 11 | - | - | 0.2 | - | - | 194 MB | 1.88-1.88 | PASS |
| plain-100 | 100 | typing@80ms | 39 | 3.1 / 3.7 / 4.8 | p95 | 11 | - | - | 0.5 | - | - | 194 MB | 1.88-1.89 | PASS |
| plain-100 | 100 | typing@100ms | 39 | 3.3 / 3.6 / 3.8 | p95 | 11 | - | - | 0.7 | - | - | 194 MB | 1.89-1.82 | PASS |
| plain-100 | 100 | typing@150ms | 39 | 3.8 / 4.2 / 4.5 | p95 | 11 | - | - | 0.8 | - | - | 194 MB | 1.82-1.76 | PASS |
| plain-100 | 100 | preamble | 13 | 36.4 / 37.4 / 38.7 | p95 | 400 | 37.4 | 154.9 | 1.6 | 0/13 | 100 / 100 | 194 MB | 1.76-1.77 | PASS |
| plain-100 | 100 | reopen: host's share, cold (spawn -> page 1) | 12 | 57.1 / 61.1 / 65.1 | p95 | (100, app) | - | - | - | - | - | 194 MB | 1.71-1.82 | report-only |
| plain-100 | 100 | reopen: host already listening (not counted, decision 8) | 12 | 17.1 / 19.1 / 26.6 | p95 | (100, app) | - | - | - | - | - | 194 MB | 1.71-1.82 | report-only |
| full-100 | 101 | sentence@middle | 19 | 6.4 / 8.8 / 8.9 | p95 | 11 | 8.8 | 38.7 | 0.5 | 19/19 | 2 / 2 | 285 MB | 1.82-2.0 | PASS |
| full-100 | 101 | newline@middle | 19 | 6.1 / 7.8 / 11.7 | p95 | 11 | 7.8 | 32.4 | 0.4 | 19/19 | 2 / 2 | 285 MB | 2.0-1.84 | PASS |
| full-100 | 101 | split@middle | 19 | 6.3 / 7.8 / 8.7 | p95 | 11 | 7.8 | 34.8 | 0.7 | 19/19 | 2 / 2 | 285 MB | 1.84-1.79 | PASS |
| full-100 | 101 | letter@start | 19 | 6.2 / 7.5 / 7.7 | p95 | 11 | 7.5 | 31.0 | 0.6 | 19/19 | 2 / 2 | 285 MB | 1.79-1.8 | PASS |
| full-100 | 101 | letter@middle | 19 | 6.5 / 7.6 / 9.8 | p95 | 11 | 7.6 | 35.9 | 0.8 | 19/19 | 2 / 2 | 285 MB | 1.8-1.98 | PASS |
| full-100 | 101 | letter@end | 19 | 8.7 / 9.3 / 9.7 | p95 | 11 | 5.4 | 30.4 | 0.8 | 19/19 | 3 / 3 | 285 MB | 1.98-1.75 | PASS |
| full-100 | 101 | typing@50ms | 39 | 4.9 / 6.3 / 6.5 | p95 | 11 | - | - | 0.2 | - | - | 285 MB | 1.75-1.75 | PASS |
| full-100 | 101 | typing@60ms | 39 | 4.9 / 5.4 / 5.8 | p95 | 11 | - | - | 0.2 | - | - | 285 MB | 1.75-1.61 | PASS |
| full-100 | 101 | typing@80ms | 39 | 5.1 / 5.6 / 5.7 | p95 | 11 | - | - | 0.4 | - | - | 285 MB | 1.61-1.61 | PASS |
| full-100 | 101 | typing@100ms | 39 | 5.5 / 5.8 / 6.1 | p95 | 11 | - | - | 0.5 | - | - | 285 MB | 1.61-1.48 | PASS |
| full-100 | 101 | typing@150ms | 39 | 6.5 / 7.0 / 9.6 | p95 | 11 | - | - | 0.8 | - | - | 285 MB | 1.48-1.44 | PASS |
| full-100 | 101 | preamble | 13 | 171.7 / 177.1 / 178.3 | p95 | 400 | 177.1 | 611.8 | 2.9 | 0/13 | 101 / 101 | 285 MB | 1.44-1.85 | PASS |
| full-100 | 101 | reopen: host's share, cold (spawn -> page 1) | 12 | 94.2 / 99.7 / 122.0 | p95 | (100, app) | - | - | - | - | - | 285 MB | 1.85-2.03 | report-only |
| full-100 | 101 | reopen: host already listening (not counted, decision 8) | 12 | 54.1 / 58.1 / 67.9 | p95 | (100, app) | - | - | - | - | - | 285 MB | 1.85-2.03 | report-only |
| plain-300 | 301 | newline@middle | 19 | 3.6 / 7.8 / 8.2 | p95 | 11 | 7.8 | 37.1 | 4.4 | 19/19 | 4 / 4 | 284 MB | 2.03-2.03 | PASS |
| plain-300 | 301 | split@middle | 19 | 3.6 / 4.8 / 4.8 | p95 | 11 | 4.8 | 42.6 | 0.6 | 19/19 | 9 / 9 | 284 MB | 2.03-2.03 | PASS |
| plain-300 | 301 | letter@start | 19 | 3.4 / 4.8 / 5.5 | p95 | 11 | 4.8 | 22.6 | 0.6 | 19/19 | 2 / 2 | 284 MB | 2.03-2.18 | PASS |
| plain-300 | 301 | letter@middle | 19 | 3.8 / 5.2 / 5.3 | p95 | 11 | 5.2 | 21.5 | 0.8 | 19/19 | 2 / 2 | 284 MB | 2.18-1.85 | PASS |
| plain-300 | 301 | letter@end | 19 | 4.3 / 5.1 / 5.2 | p95 | 11 | 5.1 | 18.3 | 0.8 | 19/19 | 2 / 2 | 284 MB | 1.85-1.7 | PASS |
| plain-300 | 301 | sentence@middle | 19 | 3.6 / 4.8 / 5.6 | p95 | 11 | 4.8 | 127.3 | 0.7 | 19/19 | 65 / 65 | 284 MB | 1.7-1.64 | PASS |
| plain-300 | 301 | typing@50ms | 39 | 2.7 / 2.9 / 3.0 | p95 | 11 | - | - | 0.1 | - | - | 284 MB | 1.64-1.59 | PASS |
| plain-300 | 301 | typing@60ms | 39 | 2.7 / 2.9 / 3.0 | p95 | 11 | - | - | 0.1 | - | - | 284 MB | 1.59-1.59 | PASS |
| plain-300 | 301 | typing@80ms | 39 | 3.1 / 3.2 / 3.2 | p95 | 11 | - | - | 0.4 | - | - | 284 MB | 1.59-1.54 | PASS |
| plain-300 | 301 | typing@100ms | 39 | 3.2 / 3.5 / 3.5 | p95 | 11 | - | - | 0.7 | - | - | 284 MB | 1.54-1.42 | PASS |
| plain-300 | 301 | typing@150ms | 39 | 3.4 / 4.3 / 4.4 | p95 | 11 | - | - | 0.7 | - | - | 284 MB | 1.42-1.39 | PASS |
| plain-300 | 301 | preamble | 13 | 41.6 / 46.9 / 47.7 | p95 | 400 | 46.9 | 593.4 | 1.7 | 0/13 | 301 / 301 | 284 MB | 1.39-1.48 | PASS |
| plain-300 | 301 | reopen: host's share, cold (spawn -> page 1) | 12 | 56.4 / 57.7 / 58.9 | p95 | (100, app) | - | - | - | - | - | 284 MB | 1.48-1.87 | report-only |
| plain-300 | 301 | reopen: host already listening (not counted, decision 8) | 12 | 17.5 / 17.9 / 18.1 | p95 | (100, app) | - | - | - | - | - | 284 MB | 1.48-1.87 | report-only |
| full-300 | 299 | split@middle | 19 | 7.6 / 8.3 / 8.4 | p95 | 11 | 8.3 | 196.7 | 0.6 | 19/19 | 33 / 33 | 441 MB | 1.8-2.19 | PASS |
| full-300 | 299 | letter@start | 19 | 6.8 / 7.8 / 8.1 | p95 | 11 | 7.8 | 34.6 | 0.7 | 19/19 | 2 / 2 | 441 MB | 2.19-2.26 | PASS |
| full-300 | 299 | letter@middle | 19 | 7.6 / 8.9 / 9.3 | p95 | 11 | 8.9 | 35.4 | 0.7 | 19/19 | 2 / 2 | 441 MB | 2.26-2.15 | PASS |
| full-300 | 299 | letter@end | 19 | 9.0 / 9.6 / 9.6 | p95 | 11 | 5.7 | 32.1 | 0.6 | 19/19 | 3 / 3 | 441 MB | 2.15-1.98 | PASS |
| full-300 | 299 | sentence@middle | 19 | 7.9 / 9.2 / 10.5 | p95 | 11 | 9.2 | 218.0 | 0.6 | 19/19 | 33 / 33 | 441 MB | 1.98-3.11 | PASS |
| full-300 | 299 | newline@middle | 19 | 7.2 / 8.5 / 8.8 | p95 | 11 | 8.5 | 30.2 | 0.2 | 19/19 | 2 / 2 | 441 MB | 3.11-2.94 | PASS |
| full-300 | 299 | typing@50ms | 39 | 6.2 / 6.4 / 6.8 | p95 | 11 | - | - | 0.2 | - | - | 441 MB | 2.94-2.86 | PASS |
| full-300 | 299 | typing@60ms | 39 | 6.3 / 6.8 / 7.5 | p95 | 11 | - | - | 0.2 | - | - | 441 MB | 2.86-2.86 | PASS |
| full-300 | 299 | typing@80ms | 39 | 6.4 / 7.3 / 7.3 | p95 | 11 | - | - | 0.2 | - | - | 441 MB | 2.86-2.71 | PASS |
| full-300 | 299 | typing@100ms | 39 | 6.8 / 7.0 / 7.6 | p95 | 11 | - | - | 0.5 | - | - | 441 MB | 2.71-2.5 | PASS |
| full-300 | 299 | typing@150ms | 39 | 7.4 / 8.0 / 8.1 | p95 | 11 | - | - | 0.7 | - | - | 441 MB | 2.5-2.54 | PASS |
| full-300 | 299 | preamble | 13 | 173.8 / 175.0 / 181.7 | p95 | 400 | 175.0 | 1460.8 | 3.0 | 0/13 | 299 / 299 | 441 MB | 2.54-1.81 | PASS |
| full-300 | 299 | reopen: host's share, cold (spawn -> page 1) | 12 | 93.7 / 100.9 / 124.7 | p95 | (100, app) | - | - | - | - | - | 441 MB | 1.81-2.42 | report-only: p95 100.9 ms > 100 ms |
| full-300 | 299 | reopen: host already listening (not counted, decision 8) | 12 | 55.0 / 59.6 / 75.0 | p95 | (100, app) | - | - | - | - | - | 441 MB | 1.81-2.42 | report-only |
| plain-1000 | 1001 | letter@start | 19 | 4.3 / 5.3 / 5.7 | p95 | 11 | 5.3 | 32.9 | 0.6 | 19/19 | 2 / 2 | 574 MB | 2.42-2.55 | PASS |
| plain-1000 | 1001 | letter@middle | 19 | 5.5 / 6.7 / 7.0 | p95 | 11 | 5.3 | 27.5 | 0.6 | 19/19 | 3 / 3 | 574 MB | 2.55-2.39 | PASS |
| plain-1000 | 1001 | letter@end | 19 | 4.1 / 5.1 / 5.4 | p95 | 11 | 5.1 | 27.3 | 0.6 | 19/19 | 2 / 2 | 574 MB | 2.39-2.36 | PASS |
| plain-1000 | 1001 | sentence@middle | 19 | 5.7 / 8.1 / 8.2 | p95 | 11 | 5.9 | 3155.3 | 0.6 | 0/19 | 1503 / 1505 | 574 MB | 2.36-3.64 | PASS |
| plain-1000 | 1001 | newline@middle | 19 | 5.9 / 7.0 / 7.1 | p95 | 11 | 5.5 | 29.1 | 0.1 | 19/19 | 3 / 3 | 574 MB | 3.64-3.54 | PASS |
| plain-1000 | 1001 | split@middle | 19 | 6.6 / 7.6 / 7.7 | p95 | 11 | 6.2 | 3078.3 | 0.7 | 0/19 | 1503 / 1505 | 574 MB | 3.54-3.14 | PASS |
| plain-1000 | 1001 | typing@50ms | 39 | 4.3 / 4.6 / 4.7 | p95 | 11 | - | - | 0.1 | - | - | 574 MB | 3.14-3.14 | PASS |
| plain-1000 | 1001 | typing@60ms | 39 | 4.4 / 4.9 / 5.0 | p95 | 11 | - | - | 0.1 | - | - | 574 MB | 3.14-3.13 | PASS |
| plain-1000 | 1001 | typing@80ms | 39 | 4.5 / 4.9 / 5.2 | p95 | 11 | - | - | 0.4 | - | - | 574 MB | 3.13-3.13 | PASS |
| plain-1000 | 1001 | typing@100ms | 39 | 4.8 / 6.1 / 11.7 | p95 | 11 | - | - | 0.4 | - | - | 574 MB | 3.13-3.68 | PASS |
| plain-1000 | 1001 | typing@150ms | 39 | 5.1 / 5.9 / 10.2 | p95 | 11 | - | - | 0.7 | - | - | 574 MB | 3.68-3.38 | PASS |
| plain-1000 | 1001 | preamble | 13 | 45.6 / 47.6 / 59.4 | p95 | 400 | 47.6 | 1314.7 | 1.6 | 0/13 | 1001 / 1001 | 574 MB | 3.38-3.35 | PASS |
| plain-1000 | 1001 | reopen: host's share, cold (spawn -> page 1) | 12 | 62.1 / 63.5 / 68.8 | p95 | (100, app) | - | - | - | - | - | 574 MB | 3.32-3.55 | report-only |
| plain-1000 | 1001 | reopen: host already listening (not counted, decision 8) | 12 | 20.3 / 20.6 / 20.7 | p95 | (100, app) | - | - | - | - | - | 574 MB | 3.32-3.55 | report-only |
| full-1000 | 1002 | letter@middle | 19 | 8.5 / 9.9 / 10.1 | p95 | 11 | 9.9 | 211.0 | 0.7 | 19/19 | 33 / 33 | 821 MB | 3.51-4.46 | PASS |
| full-1000 | 1002 | letter@end | 19 | 7.8 / 8.9 / 9.3 | p95 | 11 | 6.0 | 38.3 | 0.8 | 19/19 | 3 / 3 | 821 MB | 4.46-4.18 | PASS |
| full-1000 | 1002 | sentence@middle | 19 | 8.4 / 10.1 / 10.3 | p95 | 11 | 10.1 | 216.1 | 0.8 | 19/19 | 33 / 33 | 821 MB | 4.18-4.0 | PASS |
| full-1000 | 1002 | newline@middle | 19 | 8.4 / 9.3 / 9.5 | p95 | 11 | 9.3 | 49.8 | 0.1 | 19/19 | 3 / 3 | 821 MB | 4.0-3.54 | PASS |
| full-1000 | 1002 | split@middle | 19 | 9.7 / 10.3 / 10.5 | p95 | 11 | 10.3 | 208.9 | 0.8 | 19/19 | 33 / 33 | 821 MB | 3.54-3.61 | PASS |
| full-1000 | 1002 | letter@start | 19 | 8.7 / 10.2 / 10.4 | p95 | 11 | 10.2 | 60.3 | 0.8 | 19/19 | 2 / 2 | 821 MB | 3.61-3.4 | PASS |
| full-1000 | 1002 | typing@50ms | 39 | 8.7 / 9.8 / 10.7 | p95 | 11 | - | - | 0.4 | - | - | 821 MB | 3.4-3.21 | PASS |
| full-1000 | 1002 | typing@60ms | 39 | 8.8 / 9.6 / 11.0 | p95 | 11 | - | - | 0.6 | - | - | 821 MB | 3.21-3.21 | PASS |
| full-1000 | 1002 | typing@80ms | 39 | 9.1 / 9.9 / 10.2 | p95 | 11 | - | - | 0.6 | - | - | 821 MB | 3.21-3.03 | PASS |
| full-1000 | 1002 | typing@100ms | 39 | 9.2 / 9.8 / 10.9 | p95 | 11 | - | - | 0.7 | - | - | 821 MB | 3.03-3.03 | PASS |
| full-1000 | 1002 | typing@150ms | 39 | 9.7 / 10.9 / 13.9 | p95 | 11 | - | - | 0.7 | - | - | 821 MB | 3.03-2.94 | PASS |
| full-1000 | 1002 | preamble | 13 | 191.5 / 196.4 / 205.1 | p95 | 400 | 196.4 | 4982.4 | 3.0 | 0/13 | 1002 / 1002 | 821 MB | 2.94-2.01 | PASS |
| full-1000 | 1002 | reopen: host's share, cold (spawn -> page 1) | 12 | 99.4 / 100.7 / 101.2 | p95 | (100, app) | - | - | - | - | - | 821 MB | 1.79-2.21 | report-only: p95 100.7 ms > 100 ms |
| full-1000 | 1002 | reopen: host already listening (not counted, decision 8) | 12 | 61.2 / 62.1 / 63.1 | p95 | (100, app) | - | - | - | - | - | 821 MB | 1.79-2.21 | report-only |
| beamer-5 | 5 | letter@end | 19 | 5.6 / 6.4 / 7.9 | p95 | 11 | 6.4 | 30.2 | 1.6 | 19/19 | 2 / 2 | 252 MB | 1.99-1.91 | PASS |
| beamer-5 | 5 | sentence@middle | 19 | 5.2 / 6.4 / 6.8 | p95 | 11 | 6.4 | 22.3 | 1.2 | 19/19 | 2 / 2 | 252 MB | 1.91-2.37 | PASS |
| beamer-5 | 5 | newline@middle | 19 | 5.4 / 6.3 / 6.7 | p95 | 11 | 6.3 | 32.5 | 1.1 | 19/19 | 2 / 3 | 252 MB | 2.37-3.46 | PASS |
| beamer-5 | 5 | split@middle | 19 | 5.2 / 6.2 / 6.2 | p95 | 11 | 6.2 | 44.1 | 0.7 | 19/19 | 3 / 3 | 252 MB | 3.46-3.18 | PASS |
| beamer-5 | 5 | letter@start | 19 | 5.4 / 6.1 / 6.3 | p95 | 11 | 6.1 | 21.0 | 0.7 | 19/19 | 2 / 2 | 252 MB | 3.18-3.0 | PASS |
| beamer-5 | 5 | letter@middle | 19 | 5.2 / 6.6 / 6.6 | p95 | 11 | 6.6 | 21.5 | 0.7 | 19/19 | 2 / 2 | 252 MB | 3.0-2.76 | PASS |
| beamer-5 | 5 | typing@50ms | 39 | 3.3 / 4.0 / 5.0 | p95 | 11 | - | - | 0.1 | - | - | 252 MB | 2.76-2.76 | PASS |
| beamer-5 | 5 | typing@60ms | 39 | 3.3 / 3.5 / 3.6 | p95 | 11 | - | - | 0.1 | - | - | 252 MB | 2.76-2.78 | PASS |
| beamer-5 | 5 | typing@80ms | 39 | 3.8 / 4.3 / 11.6 | p95 | 11 | - | - | 0.4 | - | - | 252 MB | 2.78-2.72 | PASS |
| beamer-5 | 5 | typing@100ms | 39 | 4.2 / 4.4 / 4.6 | p95 | 11 | - | - | 0.6 | - | - | 252 MB | 2.72-2.72 | PASS |
| beamer-5 | 5 | typing@150ms | 39 | 4.7 / 5.6 / 10.1 | p95 | 11 | - | - | 0.7 | - | - | 252 MB | 2.72-2.67 | PASS |
| beamer-5 | 5 | preamble | 13 | 237.7 / 240.9 / 248.4 | p95 | 400 | 240.9 | 285.9 | 5.7 | 0/13 | 5 / 5 | 252 MB | 2.67-2.94 | PASS |
| beamer-5 | 5 | reopen: host's share, cold (spawn -> page 1) | 12 | 89.2 / 91.3 / 95.2 | p95 | (100, app) | - | - | - | - | - | 252 MB | 2.94-2.95 | report-only |
| beamer-5 | 5 | reopen: host already listening (not counted, decision 8) | 12 | 50.7 / 52.0 / 55.0 | p95 | (100, app) | - | - | - | - | - | 252 MB | 2.94-2.95 | report-only |
| beamer-30 | 58 | sentence@middle | 19 | 8.8 / 10.7 / 11.6 | p95 | 11 | 10.7 | 45.7 | 0.7 | 19/19 | 2 / 2 | 296 MB | 2.95-2.79 | PASS |
| beamer-30 | 58 | newline@middle | 19 | 8.1 / 9.0 / 9.0 | p95 | 11 | 9.0 | 40.8 | 0.1 | 19/19 | 3 / 3 | 296 MB | 2.79-2.65 | PASS |
| beamer-30 | 58 | split@middle | 19 | 9.2 / 9.9 / 10.2 | p95 | 11 | 9.9 | 60.0 | 0.8 | 19/19 | 3 / 3 | 296 MB | 2.65-2.32 | PASS |
| beamer-30 | 58 | letter@start | 19 | 9.7 / 11.5 / 16.8 | p95 | 11 | 11.5 | 44.6 | 0.7 | 19/19 | 2 / 2 | 296 MB | 2.32-2.21 | MISS: p95 11.5 ms > 11 ms |
| beamer-30 | 58 | letter@middle | 19 | 9.4 / 10.1 / 10.1 | p95 | 11 | 10.1 | 29.8 | 0.8 | 19/19 | 2 / 2 | 296 MB | 2.21-2.04 | PASS |
| beamer-30 | 58 | letter@end | 19 | 8.9 / 10.1 / 12.2 | p95 | 11 | 10.1 | 42.2 | 0.7 | 19/19 | 2 / 2 | 296 MB | 2.04-2.09 | PASS |
| beamer-30 | 58 | typing@50ms | 39 | 7.2 / 8.6 / 14.6 | p95 | 11 | - | - | 0.1 | - | - | 296 MB | 2.09-2.09 | PASS |
| beamer-30 | 58 | typing@60ms | 39 | 7.1 / 8.3 / 8.5 | p95 | 11 | - | - | 0.1 | - | - | 296 MB | 2.09-2.0 | PASS |
| beamer-30 | 58 | typing@80ms | 39 | 7.2 / 7.7 / 8.3 | p95 | 11 | - | - | 0.4 | - | - | 296 MB | 2.0-2.0 | PASS |
| beamer-30 | 58 | typing@100ms | 39 | 7.6 / 8.0 / 8.5 | p95 | 11 | - | - | 0.4 | - | - | 296 MB | 2.0-1.92 | PASS |
| beamer-30 | 58 | typing@150ms | 39 | 8.3 / 8.8 / 9.3 | p95 | 11 | - | - | 0.7 | - | - | 296 MB | 1.92-1.77 | PASS |
| beamer-30 | 58 | preamble | 13 | 261.4 / 275.3 / 283.3 | p95 | 400 | 275.3 | 1098.7 | 11.4 | 0/13 | 58 / 58 | 296 MB | 1.77-1.69 | PASS |
| beamer-30 | 58 | reopen: host's share, cold (spawn -> page 1) | 12 | 102.9 / 112.1 / 129.6 | p95 | (100, app) | - | - | - | - | - | 296 MB | 1.55-1.73 | report-only: p95 112.1 ms > 100 ms |
| beamer-30 | 58 | reopen: host already listening (not counted, decision 8) | 12 | 64.7 / 71.7 / 90.8 | p95 | (100, app) | - | - | - | - | - | 296 MB | 1.55-1.73 | report-only |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1); typing@: the keystroke sent to the watched page from its own compile or a later one. off-CPU: of the host's time to that page, what its engine thread spent descheduled (wall minus queue minus thread CPU): load or a CPU quota, not the engine. The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.

## Owner ruling (2026-10-21)

The owner ordered this run on the M5 Pro with the machine checks skipped ("stop checking powr settings js run stuff bro"), as they had for the 2026-10-20 run, where they waived the load threshold and marked the run official. It ran with `--force --max-load 100 --wait 0` on battery. The Commander records it as the reference run for 2026-10-21, by the owner's direction. It shows 118 of 120 gated rows passing, against 108 of 120 on 2026-10-20 (#1855). The two misses are full-10 letter@start (p95 12.2 ms, p50 11.5 ms) and beamer-30 letter@start (p95 11.5 ms, p50 9.7 ms).
