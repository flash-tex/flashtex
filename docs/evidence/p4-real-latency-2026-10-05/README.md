# P4-REAL-LATENCY: T7's 13–21 ms against 77–103 ms for the same keystroke (2026-10-05)

Lane **P4-REAL-LATENCY** (kabir-claude). DESIGN.md §1.2: key → preview commit ≤ 16 ms p95, of which the
host's share is ≤ 11 ms. P4 gate: T7.

## The contradiction

On full-1000, the edited page's host share measured two ways:

- **T7** (the 10-05 review, the newline lane): **13–21 ms** p95.
- **`dl3-keys` phases, as in `live-30ms-2026-10-04`**, on main `d4ff8c234` the same day: **77–103 ms**
  p50 for isolated keystrokes. Typing every 100–150 ms gave p95 0.23–1.1 s, and 6–24 of 59
  keystrokes were not painted by their own compile.

## What each tool times (VERIFIED, by reading `t7.py`, `keyrun.py`, `dl3-keys.rs`, `resident.rs`)

Both tools time the same interval. In each case:

- **Client and host.** The same `dl3-keys` binary, as a client of a separate `flashtex-host --socket`.
- **The interval.** COMPILE written → the watched page's `PAGE` frame read.
- **The keystroke.** The same edit: `--kind letter --at 0.5`, a letter typed and deleted on page 501.
- **Pacing.** The next keystroke goes 300 ms after `DONE`. So the background re-typesetting is over,
  and `prepare_next` has prepared the next restore.
- **Host settings.** Keep-warm is the host's default, 2 s. The first keystroke of each phase is a
  warm-up and is left out.

T7 does not time a sub-interval that hides work. What it misses is a *case*: it never sends a
keystroke while background work runs. The typing phases (`--interval-ms`) do.

## Step 1: one build, the same windows, interleaved (VERIFIED)

**Setup.**
- **Build.** Main `d4ff8c234` plus #1600's Linux instruction counters.
- **Script.** `scripts/step1.sh`, run twice. Each round runs T7 letter@middle (20 keys), then `keyrun.py`
  iso / every 100 ms / every 150 ms (40 keys each), on full-1000, then the same on plain-1000.
- **Throttling.** `cg.txt` samples `flashtex.slice`'s `cpu.stat` every second; `scripts/step1tab.py`
  gives the throttled share of each window.
- **Load.** The slice was throttled in 69–100 % of the periods of round 1, and in 0 % of the
  dl3-keys windows of round 2. The round-2 T7 window was throttled in 43 % of its periods, but
  the throttling missed its keystrokes: their off-CPU p95 is 0.1 ms.

full-1000, keystroke on page 501, p50 / p95 ms (`scripts/tab2.py`):

| stage | T7 (round 2) | dl3 iso (2) | dl3 every 100 ms (2) | dl3 iso (1, throttled) | dl3 every 100 ms (1, throttled) |
|---|---|---|---|---|---|
| key → edited PAGE (client) | **16.9 / 18.8** | **17.1 / 19.3** | **20.5 / 22.5** | 26.9 / 68.8 | 33.1 / 96.0 |
| wait for the engine thread (`queue`) | 0.0 / 0.1 | 0.0 / 0.1 | 0.3 / 1.3 (test 1.2, typeset 0.7) | 0.0 / 0.1 | 3.4 / 21.1 (typeset 12.2, jump 8.8) |
| apply + move spans | 3.4 / 4.1 | 3.4 / 5.8 | 4.8 / 5.2 | 6.2 / 46.2 | 7.2 / 7.9 |
| S₀ key check (+ abandoning the stopped run) | 0.8 / 1.0 | 0.8 / 1.0 | **4.0 / 4.6** | 1.1 / 1.5 | 5.4 / 7.0 |
| restore (M instructions) | 2.4 / 2.8 (2.8) | 2.4 / 2.7 (2.8) | 2.2 / 2.5 (2.8) | 3.1 / 3.5 (3.1) | 2.3 / 9.5 (3.0 / 24.8) |
| typeset + display list + send, to the page | 9.4 / 10.5 | 9.5 / 10.2 | 9.4 / 9.9 | 14.7 / 55.6 | 14.0 / 56.4 |
| engine thread CPU to the page | 16.7 / 18.5 | 16.8 / 19.0 | 19.4 / 21.1 | 25.1 / 26.3 | 28.2 / 30.6 |
| **off-CPU** (descheduled) | 0.1 / 0.1 | 0.1 / 0.2 | 0.2 / 0.3 | 0.5 / **43.4** | 1.2 / **47.6** |
| engine instructions to the page (M) | 154.8 / 155.5 | 154.7 / 155.6 | 181.6 / 182.4 | 156.9 / 157.2 | 182.9 / 202.6 |
| keystrokes painted by their own compile | 19 / 19 | 39 / 39 | 39 / 39 | 39 / 39 | 39 / 39 |

**plain-1000.**
- Unthrottled, T7 gives 12.5 / 17.2, iso 12.2 / 13.5 and every 100 ms 12.3 / 13.9.
- Throttled, iso gives 19.0 / 64.9 and every 100 ms 24.6 / 32.6.
- When throttled, the restore grows from 2.3 to 40.7 M instructions. The next keystroke comes
  before the previous one's background work and `prepare_next` are done.

## What this shows

- **No tool disagreement (VERIFIED).** Unthrottled, T7 and the dl3 iso phase agree within
  0.5 ms p95: 18.8 vs 19.3 on full-1000, 17.2 vs 13.5 on plain-1000. The 10-05 review's T7
  row (17.3 / 19.0, build `9f29302fe`, load 1.5) matches too.
- **Most of the 77–103 ms is the PC's CPU quota (VERIFIED).**
  - `flashtex.slice` has `CPUQuota=800%`, and every agent's work on the PC shares it. When it runs
    out, every thread in it stops until the next 100 ms period. Over the slice's life, 63 % of
    its periods were throttled (`nr_throttled` 379,704 of 604,951 when this lane started).
  - The keystroke's engine work stays the same: 155–157 M instructions. But it spends 43–48 ms
    p95 descheduled.
  - It also takes more CPU time: 25 instead of 17 ms, from SMT siblings and frequency under
    load (cycles rose from 67 to 102 M).
  - A wall time on that slice measures the quota. `t7.py` now marks such a run non-reference.
- **What typing adds, unthrottled (VERIFIED; the split is by the DONE `paused_*` stages #1608 adds, and #1608 fixes it).**
  - Every other keystroke abandons the previous compile's stopped background run. That costs
    +3.2 ms on full-1000 (key check 0.8 → 4.0 ms, +27 M instructions).
  - Of that, 3.4–4.1 ms is `reattach_pending`, but only 5.7 M user instructions: it writes the
    old PDF tail back (7 MB here). The next restore reads it again and truncates the file at the
    same place.
  - Queueing behind background work is 0.3 / 1.3 ms. Every keystroke got its own page.
- **What typing adds, throttled.**
  - Queueing behind non-preemptible background work: typeset 12.2 ms p95, the jump 8.8 ms.
  - Unprepared restores: 25–41 M instructions.
  - Keystrokes then share pages: 3 of 39 in `t7try`.
- **What remains over 11 ms when nothing interferes** is the keystroke's own work:
  - full-1000: 155 M instructions, ≈ 17 ms;
  - plain-1000: 88 M instructions, ≈ 9–12 ms, with the edited page second.
  - That is lane P4-PAGE-COST's (#1597, #1600, #1602, #1603), not queueing.

## Files

- **`scripts/`.**
  - `step1.sh` (the run);
  - `step1tab.py` (throttling per window, and stages);
  - `tab2.py` (the table above);
  - `split.py` (one file's stage split).
- **`raw/step1.tgz`.**
  - `s1/`: `cg.txt`, `marks.txt`, the dl3-keys JSONL of both rounds, the T7 runs;
  - `t7try/`: the new T7 typing rows, run fully throttled, as a demonstration of the
    non-reference verdict.
