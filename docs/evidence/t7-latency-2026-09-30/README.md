# T7 latency gate: first run (2026-09-30)

Lane T7-LATENCY-GATE (DESIGN.md §1.2, §8 T7). Tool: `tools/latency-bench` (MIT),
host: `flashtex-host` built from main at 02dcf9d07 (release). Command:

```sh
latency-bench run --host flashtex-host --pool crates/flashtex-engine/pdftex.pool \
  --docs <gen.py output> --out <dir>      # defaults: 6 local, 3 reflow, 3 preamble, 6 reopen
```

Files: `summary.md` (the table below), `summary.json` (with the machine, `uptime`
at start and end, the host's HELLO), `samples.jsonl.gz` (all 536 compiles, each
with the host's `DONE` fields, `load1`/`load5` and the busy runner count).

## The machine was saturated: these are not verdicts on the engine

mac-m1max-a (Apple M1 Max, 10 cores, 32 GB, macOS 26), 02:50–04:30 local. The
1-minute load average stayed between **101 and 1,004** (mostly 350–780) on 10
cores the whole run, and all 3 self-hosted CI runners were busy at every sample.
At 35–80 runnable threads per core, every millisecond figure here is dominated by
scheduling. The P4-L2-L3 evidence measured the same plain documents at 2–13 ms p95
(load 4–21) on another Mac. The gate's verdict on this run, 28 of 32 rows failing,
is an honest record of this run, not a claim about the engine on a quiet machine.

## Results (client side: `COMPILE` written to the frame decoded)

| doc | pages | local p50 / p95 | reflow p50 / p95 | preamble p50 / p95 | reopen p50 / p95 | load1 |
|---|---|---|---|---|---|---|
| target | | ≤ 16 p95 | ≤ 16 p95 | ≤ 400 p95 | ≤ 100 p95 | |
| plain-10 | 10 | 30.7 / 87.6 | 40.0 / 116.4 | 265.1 / **318.1** | 71.9 / 106.0 | 669–711 |
| full-10 | 10 | 88.0 / 173.3 | 61.3 / 207.6 | 2,942 / 3,966 | 273.0 / 420.6 | 717–789 |
| plain-100 | 100 | 37.6 / 80.9 | 10.1 / 63.3 | 160.9 / **389.4** | 29.8 / **89.9** | 714–778 |
| full-100 | 100 | 23.9 / 67.0 | 29.1 / 67.6 | 894.0 / 1,271 | 308.3 / 589.1 | 345–654 |
| plain-300 | 301 | 26.9 / 107.5 | 41.7 / 133.5 | 351.1 / 652.2 | 63.9 / 119.2 | 396–487 |
| full-300 | 296 | 151.4 / 659.5 | 104.9 / 1,136 | 1,499 / 4,985 | 258.4 / 1,365 | 408–784 |
| plain-1000 | 1,001 | 66.5 / 183.0 | 211.5 / 687.2 | 5,833 / 7,600 | 62.1 / 110.4 | 102–1,004 |
| full-1000 | 991 | 98.7 / 549.5 | 271.8 / 803.6 | 10,527 / 20,900 | 316.7 / 693.9 | 279–628 |

n = 36 local, 18 reflow, 6 preamble, 6 reopen per document; no page ever failed
to arrive. The host's own `first_page_ms` is within a few ms of the client's
figure in every row, so the socket and decoding are not where the time goes.

## Findings worth following up (they do not depend on the load)

1. **A preamble edit does not get viewport-first on large documents.** The first
   compile of plain-1000 shows page 1 in 251 ms and full-1000 in 624 ms, but a
   preamble edit of the same documents (a cold run: `main.tex changed in the
   110 bytes read before S0`, `viewport: 0`) shows page 1 only after 3.3–7.6 s and
   6.5–20.9 s. The delay grows with the page count (plain: 10 pages 265 ms p50,
   100 → 161, 300 → 351, 1,000 → 5,833), so something proportional to the old
   run's size (1,000 checkpoints, pages, spans) happens before page 1.
2. **Edits far from the end cost more on long documents.** plain-1000 local p50
   by region: start 82.8, middle 79.6, end 28.0 ms; full-1000: 111, 184, 29.7 ms.
   The restart is the edited page in 30–36 of 36 edits, so this is not more
   pages re-typeset; DESIGN §2's restore of far-back checkpoints (5.4–18.5 ms
   serial, 8 parallel workers) competing for cores is the likely cause.
3. **Reopen uses S₀ every time** (`mode: open` in all 48 reopens), after an edit
   on disk. The target counts `COMPILE` to page 1; starting the host process
   (format check and warm-up) took another 0.9–2.8 s median at this load.
4. **Reflow:** the edited page arrives first and the later pages follow in the
   background (5, 53, 161–165 and 542–549 later frames at 10/100/300/1,000
   pages); the background pass took 0.3–47 s median here.
5. `.aux` reruns never appeared: the settle compile after every measured one
   returned `unchanged` at once (the generated documents' labels do not move
   pages).
