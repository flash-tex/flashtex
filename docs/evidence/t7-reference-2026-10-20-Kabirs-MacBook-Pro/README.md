# T7 reference run, 2026-10-20, Kabirs-MacBook-Pro

P4 gate (DESIGN.md §12; Q1 decided 2026-10-05): `scripts/t7-reference.sh`, the full T7 run on an
idle, plugged-in Apple-Silicon Mac with Low Power Mode off.

- **Verdict:** T7: 12 MISSES (108 of 120 gated rows pass; reference run)
- **Exit:** 1: a gated target missed (the table's MISS rows)
- **Commit:** `fcc32b779` (Merge pull request #1852 from flash-tex/agent/kabir-claude/t7-beamer-memo)
- **Machine:** Mac17,9 Apple M5 Pro 26.6.2
- **Command:** `scripts/t7-reference.sh --no-build --max-load 100 --wait 0`, which ran `t7.py --engine ref --out /tmp/flashtex-t7-reference/t7-reference-run --max-load 100 --wait-load 100 --wait-max 0 --require-reference`

Files: `table.md` (the rows), `summary.json.gz` (`t7.py --check` re-evaluates it after
gunzip), `raw.tgz` (dl3-keys' lines), `environment.txt`, `t7-output.txt`.

Reference run.

| doc | pages | edit | n | ms p50 / p95 / max | gated on | target | first changed page p95 | DONE p95 ms | off-CPU p95 ms | converged | re-typeset pages p50 / max | host RSS peak | load1 | verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| plain-10 | 10 | letter@start | 19 | 6.4 / 7.7 / 8.5 | p95 | 11 | 7.7 | 21.3 | 1.3 | 19/19 | 2 / 2 | 160 MB | 1.5-1.35 | PASS |
| plain-10 | 10 | letter@middle | 19 | 4.3 / 6.1 / 6.1 | p95 | 11 | 4.6 | 18.4 | 0.7 | 19/19 | 3 / 3 | 160 MB | 1.35-1.24 | PASS |
| plain-10 | 10 | letter@end | 19 | 3.2 / 4.7 / 5.0 | p95 | 11 | 4.7 | 9.9 | 0.7 | 0/19 | 1 / 1 | 160 MB | 1.24-1.22 | PASS |
| plain-10 | 10 | sentence@middle | 19 | 4.0 / 5.6 / 5.6 | p95 | 11 | 3.9 | 21.7 | 0.7 | 0/19 | 6 / 6 | 160 MB | 1.22-1.03 | PASS |
| plain-10 | 10 | newline@middle | 19 | 4.1 / 4.9 / 5.0 | p95 | 11 | 3.3 | 17.7 | 0.0 | 19/19 | 3 / 3 | 160 MB | 1.03-1.03 | PASS |
| plain-10 | 10 | split@middle | 19 | 4.6 / 5.4 / 5.7 | p95 | 11 | 3.9 | 22.7 | 0.7 | 0/19 | 6 / 6 | 160 MB | 1.03-1.03 | PASS |
| plain-10 | 10 | typing@50ms | 39 | 3.1 / 3.5 / 6.5 | p95 | 11 | - | - | 0.0 | - | - | 160 MB | 1.03-0.94 | PASS |
| plain-10 | 10 | typing@60ms | 39 | 3.1 / 3.5 / 3.6 | p95 | 11 | - | - | 0.0 | - | - | 160 MB | 0.94-0.94 | PASS |
| plain-10 | 10 | typing@80ms | 39 | 3.6 / 3.9 / 4.0 | p95 | 11 | - | - | 0.3 | - | - | 160 MB | 0.94-0.87 | PASS |
| plain-10 | 10 | typing@100ms | 39 | 4.0 / 4.2 / 10.2 | p95 | 11 | - | - | 0.6 | - | - | 160 MB | 0.87-0.87 | PASS |
| plain-10 | 10 | typing@150ms | 39 | 4.3 / 4.9 / 5.2 | p95 | 11 | - | - | 0.7 | - | - | 160 MB | 0.87-0.81 | PASS |
| plain-10 | 10 | preamble | 13 | 34.2 / 35.6 / 35.9 | p95 | 400 | 35.6 | 47.9 | 0.6 | 0/13 | 10 / 10 | 160 MB | 0.81-0.82 | PASS |
| plain-10 | 10 | reopen: host's share, cold (spawn -> page 1) | 12 | 53.6 / 54.5 / 54.5 | p95 | (100, app) | - | - | - | - | - | 160 MB | 0.82-0.92 | report-only |
| plain-10 | 10 | reopen: host already listening (not counted, decision 8) | 12 | 15.8 / 16.0 / 16.6 | p95 | (100, app) | - | - | - | - | - | 160 MB | 0.82-0.92 | report-only |
| full-10 | 11 | letter@middle | 19 | 7.9 / 8.6 / 9.4 | p95 | 11 | 5.0 | 30.2 | 0.6 | 19/19 | 3 / 3 | 216 MB | 0.92-0.93 | PASS |
| full-10 | 11 | letter@end | 19 | 8.3 / 9.2 / 10.0 | p95 | 11 | 5.7 | 23.7 | 0.7 | 19/19 | 3 / 3 | 216 MB | 0.93-1.09 | PASS |
| full-10 | 11 | sentence@middle | 19 | 8.1 / 9.0 / 9.1 | p95 | 11 | 5.4 | 35.3 | 0.7 | 0/19 | 7 / 7 | 216 MB | 1.09-1.09 | PASS |
| full-10 | 11 | newline@middle | 19 | 7.1 / 8.1 / 8.7 | p95 | 11 | 4.6 | 30.1 | 0.0 | 19/19 | 3 / 3 | 216 MB | 1.09-0.92 | PASS |
| full-10 | 11 | split@middle | 19 | 8.4 / 9.3 / 9.4 | p95 | 11 | 5.6 | 32.9 | 0.7 | 19/19 | 3 / 3 | 216 MB | 0.92-0.85 | PASS |
| full-10 | 11 | letter@start | 19 | 12.3 / 13.4 / 13.8 | p95 | 11 | 13.4 | 38.1 | 1.2 | 19/19 | 2 / 2 | 216 MB | 0.85-0.86 | MISS: p95 13.4 ms > 11 ms |
| full-10 | 11 | typing@50ms | 39 | 6.2 / 7.3 / 11.5 | p95 | 11 | - | - | 0.0 | - | - | 216 MB | 0.86-0.87 | PASS |
| full-10 | 11 | typing@60ms | 39 | 6.2 / 7.6 / 11.9 | p95 | 11 | - | - | 0.0 | - | - | 216 MB | 0.87-0.87 | PASS |
| full-10 | 11 | typing@80ms | 39 | 6.6 / 9.1 / 9.4 | p95 | 11 | - | - | 0.3 | - | - | 216 MB | 0.87-0.88 | PASS |
| full-10 | 11 | typing@100ms | 39 | 6.9 / 7.4 / 7.5 | p95 | 11 | - | - | 0.4 | - | - | 216 MB | 0.88-0.81 | PASS |
| full-10 | 11 | typing@150ms | 39 | 7.7 / 8.1 / 8.2 | p95 | 11 | - | - | 0.6 | - | - | 216 MB | 0.81-0.82 | PASS |
| full-10 | 11 | preamble | 13 | 160.7 / 162.8 / 164.6 | p95 | 400 | 162.8 | 196.8 | 0.8 | 0/13 | 11 / 11 | 216 MB | 0.82-0.76 | PASS |
| full-10 | 11 | reopen: host's share, cold (spawn -> page 1) | 12 | 84.2 / 84.6 / 84.9 | p95 | (100, app) | - | - | - | - | - | 216 MB | 0.78-0.79 | report-only |
| full-10 | 11 | reopen: host already listening (not counted, decision 8) | 12 | 46.7 / 47.4 / 47.5 | p95 | (100, app) | - | - | - | - | - | 216 MB | 0.78-0.79 | report-only |
| plain-100 | 100 | letter@end | 19 | 5.0 / 5.7 / 6.0 | p95 | 11 | 4.2 | 17.1 | 0.7 | 19/19 | 3 / 3 | 194 MB | 0.79-0.67 | PASS |
| plain-100 | 100 | sentence@middle | 19 | 3.9 / 4.6 / 4.8 | p95 | 11 | 4.6 | 75.9 | 0.7 | 0/19 | 50 / 50 | 194 MB | 0.67-0.62 | PASS |
| plain-100 | 100 | newline@middle | 19 | 3.6 / 4.6 / 8.2 | p95 | 11 | 4.6 | 15.5 | 0.0 | 19/19 | 2 / 2 | 194 MB | 0.62-0.81 | PASS |
| plain-100 | 100 | split@middle | 19 | 4.5 / 5.2 / 5.3 | p95 | 11 | 5.2 | 18.9 | 0.7 | 19/19 | 2 / 2 | 194 MB | 0.81-0.76 | PASS |
| plain-100 | 100 | letter@start | 19 | 4.1 / 5.0 / 11.2 | p95 | 11 | 5.0 | 24.6 | 0.7 | 19/19 | 2 / 2 | 194 MB | 0.76-0.78 | PASS |
| plain-100 | 100 | letter@middle | 19 | 4.8 / 5.3 / 5.3 | p95 | 11 | 5.3 | 18.8 | 0.7 | 19/19 | 2 / 2 | 194 MB | 0.78-0.71 | PASS |
| plain-100 | 100 | typing@50ms | 39 | 2.6 / 3.1 / 3.2 | p95 | 11 | - | - | 0.0 | - | - | 194 MB | 0.71-0.66 | PASS |
| plain-100 | 100 | typing@60ms | 39 | 2.6 / 2.9 / 3.0 | p95 | 11 | - | - | 0.0 | - | - | 194 MB | 0.66-0.66 | PASS |
| plain-100 | 100 | typing@80ms | 39 | 3.1 / 3.3 / 3.7 | p95 | 11 | - | - | 0.4 | - | - | 194 MB | 0.66-0.6 | PASS |
| plain-100 | 100 | typing@100ms | 39 | 3.3 / 3.6 / 3.8 | p95 | 11 | - | - | 0.6 | - | - | 194 MB | 0.6-0.56 | PASS |
| plain-100 | 100 | typing@150ms | 39 | 3.8 / 4.1 / 4.1 | p95 | 11 | - | - | 0.7 | - | - | 194 MB | 0.56-0.51 | PASS |
| plain-100 | 100 | preamble | 13 | 35.7 / 37.7 / 49.5 | p95 | 400 | 37.7 | 157.5 | 0.7 | 0/13 | 100 / 100 | 194 MB | 0.51-0.47 | PASS |
| plain-100 | 100 | reopen: host's share, cold (spawn -> page 1) | 12 | 53.1 / 54.6 / 55.2 | p95 | (100, app) | - | - | - | - | - | 194 MB | 0.47-0.67 | report-only |
| plain-100 | 100 | reopen: host already listening (not counted, decision 8) | 12 | 15.5 / 16.5 / 16.9 | p95 | (100, app) | - | - | - | - | - | 194 MB | 0.47-0.67 | report-only |
| full-100 | 101 | sentence@middle | 19 | 7.0 / 7.6 / 8.0 | p95 | 11 | 7.6 | 28.0 | 0.7 | 19/19 | 2 / 2 | 236 MB | 0.62-0.6 | PASS |
| full-100 | 101 | newline@middle | 19 | 6.2 / 6.9 / 7.3 | p95 | 11 | 6.9 | 25.8 | 0.0 | 19/19 | 2 / 2 | 236 MB | 0.6-0.79 | PASS |
| full-100 | 101 | split@middle | 19 | 7.2 / 7.9 / 12.1 | p95 | 11 | 7.9 | 32.3 | 1.1 | 19/19 | 2 / 2 | 236 MB | 0.79-0.72 | PASS |
| full-100 | 101 | letter@start | 19 | 7.1 / 8.2 / 8.3 | p95 | 11 | 8.2 | 34.5 | 0.7 | 19/19 | 2 / 2 | 236 MB | 0.72-0.69 | PASS |
| full-100 | 101 | letter@middle | 19 | 6.8 / 7.6 / 7.7 | p95 | 11 | 7.6 | 27.7 | 0.7 | 19/19 | 2 / 2 | 236 MB | 0.69-0.64 | PASS |
| full-100 | 101 | letter@end | 19 | 8.4 / 9.4 / 9.4 | p95 | 11 | 5.7 | 29.1 | 0.7 | 19/19 | 3 / 3 | 236 MB | 0.64-0.67 | PASS |
| full-100 | 101 | typing@50ms | 39 | 4.8 / 5.3 / 5.7 | p95 | 11 | - | - | 0.0 | - | - | 236 MB | 0.67-0.69 | PASS |
| full-100 | 101 | typing@60ms | 39 | 4.9 / 5.8 / 10.9 | p95 | 11 | - | - | 0.0 | - | - | 236 MB | 0.69-0.69 | PASS |
| full-100 | 101 | typing@80ms | 39 | 5.1 / 5.8 / 6.0 | p95 | 11 | - | - | 0.3 | - | - | 236 MB | 0.69-0.72 | PASS |
| full-100 | 101 | typing@100ms | 39 | 5.5 / 5.8 / 8.8 | p95 | 11 | - | - | 0.5 | - | - | 236 MB | 0.72-0.82 | PASS |
| full-100 | 101 | typing@150ms | 39 | 6.1 / 6.8 / 7.1 | p95 | 11 | - | - | 0.7 | - | - | 236 MB | 0.82-0.91 | PASS |
| full-100 | 101 | preamble | 13 | 166.6 / 171.3 / 179.3 | p95 | 400 | 171.3 | 540.3 | 0.8 | 0/13 | 101 / 101 | 236 MB | 0.91-1.01 | PASS |
| full-100 | 101 | reopen: host's share, cold (spawn -> page 1) | 12 | 84.2 / 84.8 / 84.9 | p95 | (100, app) | - | - | - | - | - | 236 MB | 0.93-0.94 | report-only |
| full-100 | 101 | reopen: host already listening (not counted, decision 8) | 12 | 47.3 / 47.9 / 47.9 | p95 | (100, app) | - | - | - | - | - | 236 MB | 0.93-0.94 | report-only |
| plain-300 | 301 | newline@middle | 19 | 3.5 / 4.8 / 10.3 | p95 | 11 | 4.8 | 36.8 | 0.1 | 19/19 | 4 / 4 | 281 MB | 0.94-0.86 | PASS |
| plain-300 | 301 | split@middle | 19 | 4.4 / 5.3 / 11.1 | p95 | 11 | 5.3 | 41.6 | 0.7 | 19/19 | 9 / 9 | 281 MB | 0.86-0.88 | PASS |
| plain-300 | 301 | letter@start | 19 | 4.7 / 5.4 / 5.4 | p95 | 11 | 5.4 | 21.7 | 0.7 | 19/19 | 2 / 2 | 281 MB | 0.88-0.89 | PASS |
| plain-300 | 301 | letter@middle | 19 | 3.9 / 5.1 / 5.2 | p95 | 11 | 5.1 | 19.8 | 0.7 | 19/19 | 2 / 2 | 281 MB | 0.89-0.82 | PASS |
| plain-300 | 301 | letter@end | 19 | 4.2 / 5.1 / 5.2 | p95 | 11 | 5.1 | 18.0 | 0.7 | 19/19 | 2 / 2 | 281 MB | 0.82-0.85 | PASS |
| plain-300 | 301 | sentence@middle | 19 | 4.5 / 5.3 / 5.4 | p95 | 11 | 5.3 | 116.7 | 0.7 | 19/19 | 65 / 65 | 281 MB | 0.85-0.78 | PASS |
| plain-300 | 301 | typing@50ms | 39 | 2.7 / 3.1 / 3.4 | p95 | 11 | - | - | 0.0 | - | - | 281 MB | 0.78-0.88 | PASS |
| plain-300 | 301 | typing@60ms | 39 | 2.7 / 3.0 / 7.8 | p95 | 11 | - | - | 0.0 | - | - | 281 MB | 0.88-0.88 | PASS |
| plain-300 | 301 | typing@80ms | 39 | 3.1 / 3.3 / 3.4 | p95 | 11 | - | - | 0.3 | - | - | 281 MB | 0.88-0.81 | PASS |
| plain-300 | 301 | typing@100ms | 39 | 3.2 / 3.5 / 3.7 | p95 | 11 | - | - | 0.4 | - | - | 281 MB | 0.81-0.82 | PASS |
| plain-300 | 301 | typing@150ms | 39 | 3.5 / 4.1 / 4.3 | p95 | 11 | - | - | 0.6 | - | - | 281 MB | 0.82-0.84 | PASS |
| plain-300 | 301 | preamble | 13 | 36.4 / 38.7 / 49.4 | p95 | 400 | 38.7 | 397.6 | 0.7 | 0/13 | 301 / 301 | 281 MB | 0.84-0.86 | PASS |
| plain-300 | 301 | reopen: host's share, cold (spawn -> page 1) | 12 | 53.8 / 56.5 / 56.6 | p95 | (100, app) | - | - | - | - | - | 281 MB | 0.86-1.04 | report-only |
| plain-300 | 301 | reopen: host already listening (not counted, decision 8) | 12 | 16.1 / 17.2 / 18.4 | p95 | (100, app) | - | - | - | - | - | 281 MB | 0.86-1.04 | report-only |
| full-300 | 299 | split@middle | 19 | 7.8 / 8.8 / 9.2 | p95 | 11 | 8.8 | 184.3 | 0.6 | 19/19 | 33 / 33 | 340 MB | 1.03-1.03 | PASS |
| full-300 | 299 | letter@start | 19 | 7.2 / 8.2 / 11.4 | p95 | 11 | 8.2 | 32.9 | 0.7 | 19/19 | 2 / 2 | 340 MB | 1.03-0.95 | PASS |
| full-300 | 299 | letter@middle | 19 | 7.9 / 8.7 / 8.9 | p95 | 11 | 8.7 | 31.7 | 0.7 | 19/19 | 2 / 2 | 340 MB | 0.95-0.87 | PASS |
| full-300 | 299 | letter@end | 19 | 8.6 / 9.3 / 9.8 | p95 | 11 | 5.6 | 30.8 | 0.7 | 19/19 | 3 / 3 | 340 MB | 0.87-0.88 | PASS |
| full-300 | 299 | sentence@middle | 19 | 8.2 / 9.5 / 9.7 | p95 | 11 | 9.5 | 176.6 | 0.7 | 19/19 | 33 / 33 | 340 MB | 0.88-0.89 | PASS |
| full-300 | 299 | newline@middle | 19 | 7.7 / 8.5 / 8.6 | p95 | 11 | 8.5 | 29.5 | 0.0 | 19/19 | 2 / 2 | 340 MB | 0.89-0.91 | PASS |
| full-300 | 299 | typing@50ms | 39 | 6.0 / 6.5 / 6.6 | p95 | 11 | - | - | 0.0 | - | - | 340 MB | 0.91-0.91 | PASS |
| full-300 | 299 | typing@60ms | 39 | 6.1 / 6.8 / 7.1 | p95 | 11 | - | - | 0.0 | - | - | 340 MB | 0.91-0.84 | PASS |
| full-300 | 299 | typing@80ms | 39 | 6.3 / 6.9 / 7.7 | p95 | 11 | - | - | 0.3 | - | - | 340 MB | 0.84-0.84 | PASS |
| full-300 | 299 | typing@100ms | 39 | 6.6 / 7.4 / 7.7 | p95 | 11 | - | - | 0.3 | - | - | 340 MB | 0.84-0.77 | PASS |
| full-300 | 299 | typing@150ms | 39 | 7.4 / 8.1 / 8.3 | p95 | 11 | - | - | 0.6 | - | - | 340 MB | 0.77-0.79 | PASS |
| full-300 | 299 | preamble | 13 | 171.3 / 173.9 / 189.6 | p95 | 400 | 173.9 | 1354.8 | 2.6 | 0/13 | 299 / 299 | 340 MB | 0.79-0.93 | PASS |
| full-300 | 299 | reopen: host's share, cold (spawn -> page 1) | 12 | 87.0 / 90.1 / 93.0 | p95 | (100, app) | - | - | - | - | - | 340 MB | 0.86-1.01 | report-only |
| full-300 | 299 | reopen: host already listening (not counted, decision 8) | 12 | 50.2 / 53.2 / 55.2 | p95 | (100, app) | - | - | - | - | - | 340 MB | 0.86-1.01 | report-only |
| plain-1000 | 1001 | letter@start | 19 | 5.6 / 6.3 / 7.8 | p95 | 11 | 6.3 | 31.4 | 0.7 | 19/19 | 2 / 2 | 562 MB | 0.86-1.04 | PASS |
| plain-1000 | 1001 | letter@middle | 19 | 6.0 / 7.0 / 7.1 | p95 | 11 | 5.6 | 26.4 | 0.7 | 19/19 | 3 / 3 | 562 MB | 1.04-1.12 | PASS |
| plain-1000 | 1001 | letter@end | 19 | 5.2 / 5.9 / 5.9 | p95 | 11 | 5.9 | 20.2 | 0.7 | 19/19 | 2 / 2 | 562 MB | 1.12-1.03 | PASS |
| plain-1000 | 1001 | sentence@middle | 19 | 5.8 / 7.5 / 7.6 | p95 | 11 | 6.2 | 2785.8 | 0.6 | 0/19 | 1503 / 1505 | 562 MB | 1.03-1.5 | PASS |
| plain-1000 | 1001 | newline@middle | 19 | 6.1 / 7.1 / 7.5 | p95 | 11 | 5.7 | 27.9 | 0.0 | 19/19 | 3 / 3 | 562 MB | 1.5-1.42 | PASS |
| plain-1000 | 1001 | split@middle | 19 | 6.9 / 8.4 / 8.5 | p95 | 11 | 7.1 | 2729.0 | 0.7 | 0/19 | 1503 / 1505 | 562 MB | 1.42-2.06 | PASS |
| plain-1000 | 1001 | typing@50ms | 39 | 4.3 / 4.9 / 8.9 | p95 | 11 | - | - | 0.0 | - | - | 562 MB | 2.06-2.06 | PASS |
| plain-1000 | 1001 | typing@60ms | 39 | 4.3 / 4.5 / 4.9 | p95 | 11 | - | - | 0.0 | - | - | 562 MB | 2.06-2.06 | PASS |
| plain-1000 | 1001 | typing@80ms | 39 | 4.4 / 4.8 / 9.0 | p95 | 11 | - | - | 0.3 | - | - | 562 MB | 2.06-2.06 | PASS |
| plain-1000 | 1001 | typing@100ms | 39 | 4.8 / 5.1 / 5.3 | p95 | 11 | - | - | 0.3 | - | - | 562 MB | 2.06-2.05 | PASS |
| plain-1000 | 1001 | typing@150ms | 39 | 5.1 / 6.0 / 6.2 | p95 | 11 | - | - | 0.6 | - | - | 562 MB | 2.05-1.97 | PASS |
| plain-1000 | 1001 | preamble | 13 | 43.1 / 44.1 / 46.7 | p95 | 400 | 44.1 | 1271.0 | 0.6 | 0/13 | 1001 / 1001 | 562 MB | 1.97-1.82 | PASS |
| plain-1000 | 1001 | reopen: host's share, cold (spawn -> page 1) | 12 | 54.5 / 56.5 / 63.1 | p95 | (100, app) | - | - | - | - | - | 562 MB | 1.77-1.88 | report-only |
| plain-1000 | 1001 | reopen: host already listening (not counted, decision 8) | 12 | 17.5 / 17.8 / 18.8 | p95 | (100, app) | - | - | - | - | - | 562 MB | 1.77-1.88 | report-only |
| full-1000 | 1002 | letter@middle | 19 | 8.1 / 9.6 / 9.7 | p95 | 11 | 9.6 | 197.1 | 0.7 | 19/19 | 33 / 33 | 660 MB | 1.81-1.78 | PASS |
| full-1000 | 1002 | letter@end | 19 | 8.3 / 9.9 / 9.9 | p95 | 11 | 6.8 | 37.4 | 0.7 | 19/19 | 3 / 3 | 660 MB | 1.78-1.79 | PASS |
| full-1000 | 1002 | sentence@middle | 19 | 8.9 / 10.0 / 10.1 | p95 | 11 | 10.0 | 181.6 | 0.7 | 19/19 | 33 / 33 | 660 MB | 1.79-1.52 | PASS |
| full-1000 | 1002 | newline@middle | 19 | 8.4 / 9.7 / 12.5 | p95 | 11 | 9.7 | 52.2 | 0.0 | 19/19 | 3 / 3 | 660 MB | 1.52-1.4 | PASS |
| full-1000 | 1002 | split@middle | 19 | 9.1 / 9.9 / 10.5 | p95 | 11 | 9.9 | 183.9 | 0.7 | 19/19 | 33 / 33 | 660 MB | 1.4-1.25 | PASS |
| full-1000 | 1002 | letter@start | 19 | 9.5 / 10.8 / 11.0 | p95 | 11 | 10.8 | 47.8 | 0.7 | 19/19 | 2 / 2 | 660 MB | 1.25-1.13 | PASS |
| full-1000 | 1002 | typing@50ms | 39 | 8.6 / 10.5 / 14.9 | p95 | 11 | - | - | 0.2 | - | - | 660 MB | 1.13-1.13 | PASS |
| full-1000 | 1002 | typing@60ms | 39 | 8.7 / 10.3 / 11.4 | p95 | 11 | - | - | 0.4 | - | - | 660 MB | 1.13-1.12 | PASS |
| full-1000 | 1002 | typing@80ms | 39 | 9.0 / 10.1 / 11.3 | p95 | 11 | - | - | 0.5 | - | - | 660 MB | 1.12-1.12 | PASS |
| full-1000 | 1002 | typing@100ms | 39 | 9.2 / 12.9 / 13.4 | p95 | 11 | - | - | 0.5 | - | - | 660 MB | 1.12-1.11 | MISS: p95 12.9 ms > 11 ms |
| full-1000 | 1002 | typing@150ms | 39 | 9.6 / 11.9 / 18.1 | p95 | 11 | - | - | 0.7 | - | - | 660 MB | 1.11-1.1 | MISS: p95 11.9 ms > 11 ms |
| full-1000 | 1002 | preamble | 13 | 187.3 / 189.3 / 205.4 | p95 | 400 | 189.3 | 4247.1 | 0.8 | 0/13 | 1002 / 1002 | 660 MB | 1.1-1.35 | PASS |
| full-1000 | 1002 | reopen: host's share, cold (spawn -> page 1) | 12 | 94.6 / 98.1 / 101.9 | p95 | (100, app) | - | - | - | - | - | 660 MB | 1.56-1.96 | report-only |
| full-1000 | 1002 | reopen: host already listening (not counted, decision 8) | 12 | 57.7 / 61.2 / 63.0 | p95 | (100, app) | - | - | - | - | - | 660 MB | 1.56-1.96 | report-only |
| beamer-5 | 5 | letter@end | 19 | 5.7 / 6.5 / 6.9 | p95 | 11 | 6.5 | 29.3 | 0.7 | 19/19 | 2 / 2 | 250 MB | 1.84-1.77 | PASS |
| beamer-5 | 5 | sentence@middle | 19 | 5.7 / 7.0 / 7.1 | p95 | 11 | 7.0 | 22.7 | 0.7 | 19/19 | 2 / 2 | 250 MB | 1.77-1.73 | PASS |
| beamer-5 | 5 | newline@middle | 19 | 5.0 / 5.6 / 5.8 | p95 | 11 | 5.6 | 26.5 | 0.0 | 19/19 | 2 / 3 | 250 MB | 1.73-1.83 | PASS |
| beamer-5 | 5 | split@middle | 19 | 6.0 / 7.0 / 7.3 | p95 | 11 | 7.0 | 46.6 | 0.7 | 19/19 | 3 / 3 | 250 MB | 1.83-2.16 | PASS |
| beamer-5 | 5 | letter@start | 19 | 5.7 / 6.7 / 6.8 | p95 | 11 | 6.7 | 22.1 | 0.7 | 19/19 | 2 / 2 | 250 MB | 2.16-1.83 | PASS |
| beamer-5 | 5 | letter@middle | 19 | 5.5 / 7.2 / 7.4 | p95 | 11 | 7.2 | 22.8 | 0.7 | 19/19 | 2 / 2 | 250 MB | 1.83-1.68 | PASS |
| beamer-5 | 5 | typing@50ms | 39 | 3.8 / 4.3 / 4.9 | p95 | 11 | - | - | 0.0 | - | - | 250 MB | 1.68-1.68 | PASS |
| beamer-5 | 5 | typing@60ms | 39 | 3.9 / 4.3 / 4.7 | p95 | 11 | - | - | 0.0 | - | - | 250 MB | 1.68-1.55 | PASS |
| beamer-5 | 5 | typing@80ms | 39 | 4.4 / 5.0 / 11.3 | p95 | 11 | - | - | 0.3 | - | - | 250 MB | 1.55-1.42 | PASS |
| beamer-5 | 5 | typing@100ms | 39 | 4.6 / 5.1 / 5.6 | p95 | 11 | - | - | 0.5 | - | - | 250 MB | 1.42-1.42 | PASS |
| beamer-5 | 5 | typing@150ms | 39 | 5.4 / 5.9 / 6.0 | p95 | 11 | - | - | 0.7 | - | - | 250 MB | 1.42-1.28 | PASS |
| beamer-5 | 5 | preamble | 13 | 232.4 / 238.7 / 299.8 | p95 | 400 | 238.7 | 284.0 | 0.8 | 0/13 | 5 / 5 | 250 MB | 1.28-1.34 | PASS |
| beamer-5 | 5 | reopen: host's share, cold (spawn -> page 1) | 12 | 83.9 / 87.5 / 89.8 | p95 | (100, app) | - | - | - | - | - | 250 MB | 1.34-1.47 | report-only |
| beamer-5 | 5 | reopen: host already listening (not counted, decision 8) | 12 | 45.3 / 46.2 / 49.0 | p95 | (100, app) | - | - | - | - | - | 250 MB | 1.34-1.47 | report-only |
| beamer-30 | 58 | sentence@middle | 19 | 11.8 / 13.0 / 13.2 | p95 | 11 | 13.0 | 44.7 | 0.7 | 19/19 | 2 / 2 | 287 MB | 1.35-1.22 | MISS: p95 13.0 ms > 11 ms |
| beamer-30 | 58 | newline@middle | 19 | 10.2 / 11.8 / 16.1 | p95 | 11 | 11.8 | 46.3 | 0.0 | 19/19 | 3 / 3 | 287 MB | 1.22-1.12 | MISS: p95 11.8 ms > 11 ms |
| beamer-30 | 58 | split@middle | 19 | 11.5 / 12.6 / 13.0 | p95 | 11 | 12.6 | 63.6 | 0.7 | 19/19 | 3 / 3 | 287 MB | 1.12-1.1 | MISS: p95 12.6 ms > 11 ms |
| beamer-30 | 58 | letter@start | 19 | 13.2 / 14.9 / 17.8 | p95 | 11 | 14.9 | 49.3 | 0.7 | 19/19 | 2 / 2 | 287 MB | 1.1-1.01 | MISS: p95 14.9 ms > 11 ms |
| beamer-30 | 58 | letter@middle | 19 | 11.1 / 12.4 / 12.7 | p95 | 11 | 12.4 | 34.8 | 0.7 | 19/19 | 2 / 2 | 287 MB | 1.01-0.93 | MISS: p95 12.4 ms > 11 ms |
| beamer-30 | 58 | letter@end | 19 | 11.1 / 12.4 / 17.1 | p95 | 11 | 12.4 | 43.8 | 0.9 | 19/19 | 2 / 2 | 287 MB | 0.93-0.85 | MISS: p95 12.4 ms > 11 ms |
| beamer-30 | 58 | typing@50ms | 39 | 9.1 / 11.1 / 16.0 | p95 | 11 | - | - | 0.0 | - | - | 287 MB | 0.85-0.85 | MISS: p95 11.1 ms > 11 ms |
| beamer-30 | 58 | typing@60ms | 39 | 9.6 / 14.6 / 17.3 | p95 | 11 | - | - | 4.7 | - | - | 287 MB | 0.85-0.87 | MISS: p95 14.6 ms > 11 ms |
| beamer-30 | 58 | typing@80ms | 39 | 9.5 / 10.7 / 10.9 | p95 | 11 | - | - | 0.3 | - | - | 287 MB | 0.87-0.87 | PASS |
| beamer-30 | 58 | typing@100ms | 39 | 9.9 / 10.9 / 11.5 | p95 | 11 | - | - | 0.3 | - | - | 287 MB | 0.87-0.88 | PASS |
| beamer-30 | 58 | typing@150ms | 39 | 10.5 / 11.2 / 11.5 | p95 | 11 | - | - | 0.6 | - | - | 287 MB | 0.88-0.81 | MISS: p95 11.2 ms > 11 ms |
| beamer-30 | 58 | preamble | 13 | 256.7 / 260.4 / 269.1 | p95 | 400 | 260.4 | 1063.2 | 0.8 | 0/13 | 58 / 58 | 287 MB | 0.81-1.0 | PASS |
| beamer-30 | 58 | reopen: host's share, cold (spawn -> page 1) | 12 | 96.4 / 100.6 / 114.9 | p95 | (100, app) | - | - | - | - | - | 287 MB | 1.14-1.21 | report-only: p95 100.6 ms > 100 ms |
| beamer-30 | 58 | reopen: host already listening (not counted, decision 8) | 12 | 59.0 / 62.4 / 63.0 | p95 | (100, app) | - | - | - | - | - | 287 MB | 1.14-1.21 | report-only |

ms: COMPILE written to the watched page's PAGE frame read on the socket (preamble, reopen: page 1); typing@: the keystroke sent to the watched page from its own compile or a later one. off-CPU: of the host's time to that page, what its engine thread spent descheduled (wall minus queue minus thread CPU): load or a CPU quota, not the engine. The first sample of each phase is a warm-up, left out; rows with n < 12 are gated on their maximum. No noise margin: a row passes only when it meets its target.

## Owner ruling on this run (2026-10-20)

The owner ran this on their M5 Pro on mains power and ruled that it counts as the
official reference run: "fuck the load stuff. js run it now and mark it as official".
The load threshold was therefore waived (`--max-load 100 --wait 0`); load1 stayed at
about 0.8–1.5 during the run. Every other machine check passed (Apple Silicon, mains
power, Low Power Mode off, no thermal limit, clean checkout, TeX Live 2026).
