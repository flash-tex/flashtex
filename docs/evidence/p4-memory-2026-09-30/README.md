# P4-MEMORY: the resident host within its memory budget (2026-09-30)

Lane **P4-MEMORY** (kabir-claude, mac-m5pro-kabir): DESIGN.md §5.2 ("the budget drives the
policy", default 1 GB), §1.2, §7. Branch `agent/kabir-claude/p4-memory`, from `origin/main`
`296c90197` with `agent/kabir-claude/p4-finish-2` merged in (#1269 had not landed).

**The problem** (P4-FINISH §10): the socket host peaked at 5–7.5 GB on plain-1000 and ~20 GB on
full-1000 and pushed the owner's laptop into swap.

**The result** (VERIFIED, NixOS PC): peak resident memory of the host while typing on four pages
of each document, 64 keystrokes after the full compile:

| doc | before | after |
|---|---|---|
| plain-10 | 251 MB | 161 MB |
| full-10 | 390 MB | 167 MB |
| plain-120 | 671 MB | 201 MB |
| full-120 | 2,848 MB | 348 MB |
| plain-1000 | 7,360 MB | 623 MB |
| full-1000 | > 14 GB (killed at the 14 GB limit after 44 s, during its first keystrokes) | 1,497 MB |
| the owner's book.tex (1,072 pages; production builds, 24 keystrokes) | > 10 GB (killed at the limit within 10 s) | 1,025 MB |

(mem-stats builds, `raw/summary-before-ms.jsonl`, `raw/summary-side-ms.jsonl`; the production
builds give the same picture, §4.) full-1000's peak is the undo logs at their 1 GB budget plus
0.47 GB of everything else: the lane's target, **≤ 1 GB of checkpoints + ≤ 0.5 GB of the rest**, is
met, at its edge on full-1000.

**Labels.** VERIFIED means measured here; the command and raw file are named. BELIEF means an
inference that was not measured.

**Machines.** The NixOS PC (Ryzen 7 7800X3D, 16 threads, 30 GB, TeX Live 2026) was shared with
other lanes' soundness and corpus runs: load 10–60 during these measurements. **Memory figures do
not depend on load; latency figures do**, and each latency table says which runs were interleaved.
The Mac (M5 Pro) confirmation ran at load 3.

## 1. Accounting: where the memory went (VERIFIED)

**Instruments** (commit `6ddce18af`):

- `crate::memstat`: the process's resident memory (Linux `VmRSS`/`VmHWM`, macOS `phys_footprint`
  and its lifetime maximum); `mincore` for the word space and the slab (mapped, not from the
  allocator); with the cargo feature `mem-stats` (measurement only) a counting global allocator
  that tags every heap allocation with the code that made it: `engine`, `record` (checkpoint host
  records), `log` (sealed undo logs and their merges), `branch` (a restore's detached run), `side`
  (the display list's side table), `cow` (copies of shared pdfTeX C state), `dl` (display lists
  and the host's page cache), `test` (the convergence test), `prepare` (the idle restore), with the
  split at the heap's peak.
- `incr::Session::mem_stats` / `Globals::mem_stats` / `Arena::mem_stats`: the parts: word space
  (reserved, written, resident), slab (mapped, live, resident), sealed logs of the chain and of the
  detached branch (bytes, deltas, words), the open log, the prepared restore, host records, the
  side table's distinct chunks, the terminal. The socket host puts it in DONE as `mem` with
  `FLASHTEX_MEMSTAT=1`; `iserve` answers `mem`.
- `tools/incr-bench/mem.py`: types through the socket as the app does (`dl3-keys`, 300 ms apart),
  samples the host's RSS every 50 ms, kills it (by PID) above `--limit-gb`, gates at `--gate-gb`.
  `mem_table.py` tabulates.

**Breakdown before** (`base-ms`, the branch with only the accounting added; MB; the last keystroke's
`mem`, and the heap's peak):

| doc | checkpoints | peak RSS | heap peak | side table (heap) | undo logs | word space res. | engine (heap) | page cache |
|---|---|---|---|---|---|---|---|---|
| plain-10 | 96 | 251 | 151 | 99 | 9 | 32 | 24 | 0.5 |
| full-10 | 121 | 390 | 291 | 188 | 15 | 37 | 25 | 0.5 |
| plain-120 | 316 | 671 | 578 | 371 | 34 | 32 | 27 | 6 |
| full-120 | 718 | 2,848 | 2,742 | 1,267 | 66 | 37 | 28 | 6 |
| plain-1000 | 3,098 | 7,360 | 7,245 | 3,446 (6,775 at the peak) | 281 | 32 | 61 | 48 |
| full-1000 | killed at 14 GB | | | | | | | |

On plain-1000 the side table held **6,775 MB of the 7,245 MB heap peak**: 216,639 distinct 16 KB
chunks across 3,098 checkpoints (`side_chunks`). Everything else in the brief's list is small:

- undo logs 281 MB (within the 1 GB budget, which they were the only thing counted against);
- host records (`ExtRecord`) 0.3 MB; their file lines 0.35 MB; the slab 3–7 MB; the prepared
  restore 0 at DONE (it is made after); the detached branch's records 3 MB; the terminal 8 MB;
- pdfTeX's C state (fonts, maps: `engine`) 24–61 MB, and its copy-on-write copies (`cow`) ≤ 1 MB:
  font data is not copied per checkpoint (it is `Shared`, P4-L1);
- the page cache (display lists of every page) 48 MB on 1,000 pages;
- the read-set and `reloc` tables < 3 MB.

**Why the side table.** The display list's side table (a source position for every `mem`
location, `src/displaylist/`) lived outside the word space as a copy-on-write array of 16 KB chunks.
Every checkpoint shared the table; the first write to a chunk after it copied the whole chunk. A
page allocates nodes all over `mem`, so each checkpoint kept ~70 private 16 KB chunks (1.1 MB), none
of it counted against the budget, merged by retention or given back by a convergence. A restore then
kept the old run's snapshots and the new run's at once (the 6.8 GB peak).

## 2. Fixes (VERIFIED unless marked)

**The side table moves into the word space** (`7605b5ef4`). `changes/displaylist.ch` declares
`dl_side: array[mem_bot..mem_max] of memory_word` (the generated code is regenerated; the drift
test passes). The writer reads and writes `Globals::dl_side` through the write barrier, and only
writes an entry that changes. So a checkpoint keeps it as the words that changed, the budget counts
it, retention merges it, restores and the convergence jump restore it. What it is not:

- the engine's state: the convergence test leaves it out, whole chunks in `Arena::diff_branch`
  (`UNSTATED`) and the words at its boundaries in `incr::dead_word`, as it left the old table out.
  **The jump adopts it**, though, like every other array (`Arena::diff_branch_all`, `5dab984c4`):
  the side table must describe the nodes of the `mem` the jump adopts. Until then it kept the new
  run's entries for the old run's node addresses. The independent review of #1300 found this with
  `dlspan.py`: 1,252 glyphs on a wrong source line on plain-10, 241 on full-10 and 16 on plain-120,
  against 0 before the PR. Its patch is applied as written; the `span` gate (§6) now checks it;
- the intrinsics verifier ignores it (`EXCLUDED_REGIONS`);
- an S₀ file carries no side table, as before (`write_s0` zeroes its words: span numbers belong to
  the process).

Heap on full-10: 170 → 46 MB (Mac). Word-space undo logs grow by the side table's words: plain-1000
281 → 367 MB, full-120 66 → 91 MB.

**The heap's free pages go back to the system once the host is idle** (`beb7b89d1`, deferred in
`5dab984c4`). The Linux host calls `malloc_trim(0)`: glibc had kept what a compile freed mapped, so
RSS stayed at its peak. full-1000: 1,497 MB resident at rest → 592 MB.
- It took 3 ms (plain-1000), 8–10 ms (full-120) and 60–110 ms (full-1000)
  (`raw/p4mem-perkey.tgz`, `*.host-stderr`).
- It cannot be interrupted, so it now runs only once the keep-warm window has ended and 2 more
  seconds have passed without a request: 4 s after the last compile by default. It never runs
  between keystrokes (review of #1300, item 4).
- The peak is unchanged. macOS's allocator returns pages itself (BELIEF: not measured separately).
  `FLASHTEX_NO_TRIM=1` turns it off.

**A restore reads only the output tail, into the last restore's buffer** (`e545086f4`, Linux). The
review measured plain-1000's `restore` stage p95 14 → 24 ms against the base. Split per keystroke
with `FLASHTEX_INCR_DEBUG` (`scripts/restore_parts.py`, `raw/p4mem-dbg.tgz`), base against this
branch, interleaved:
- **The undo grew** from a median of 0.35 to 0.8 ms. Prepared restores copy 2,160 chunks instead of
  1,290: the side table's. This is the side table's real cost; it remains.
- **The output tails grew** from 2–5 ms to 9–12 ms at p95, with `malloc_trim` on or off.
  - `read_tail` read the whole PDF and then copied its tail: two fresh buffers of tens of MB on
    every restore.
  - The heap is now small enough that these buffers sit at its top, so glibc unmapped them on
    free and the kernel had to map and zero them again on every restore.
  - It now seeks and reads the tail alone, into the buffer of the previous restore's tail of the
    same file. There is one spare per file, 64 MB in all; a 1,000-page preview PDF is 13–14 MB.
    `e545086f4` kept a single spare, which went to the `.log`; the re-review of #1300 found this,
    and the per-file spares replaced it in the commit after `ef66c9396`.
  - macOS clones the file and was never affected.

**Retention by nested steps** (`593263746`, `e0c81d691`). `thin` (§5.2's policy) was already driven
by the logs' bytes, but its rungs `s` = 1, 2, 3, 4, 6, 8 were applied one after another, so their
spacings compounded (every 2nd page, then every 3rd of those: every 6th, then 12th). With the 1 GB
budget one pass on full-1000 ended at 119 MB and 553 checkpoints. The steps now keep nested
subsets: every page checkpoint first (only the segment checkpoints away from the cursor go), then
the spacing doubled one octave of distance at a time, the farthest first, stopping at the first that
fits in 95% of the budget. Same budget, same keystrokes: 1,282–1,306 checkpoints kept at the end
(`raw/ab-sw1024.jsonl`, `raw/ab-final.jsonl`).

**Not changed, with the reason:**

- The slab never unmaps its blocks: 3–17 MB resident.
- The page cache (48 MB at 1,000 pages) stays whole: the host serves pages from it to a client that
  reconnects or scrolls, and it is 3% of the total.
- Host records are 0.3–41 MB.
- The budget counts the undo logs and the slab's live chunks, not the records (41 MB at the peak on
  full-1000), so "checkpoints" at the budget are 1.02–1.06 GB.
- The spare output-tail buffers are not budgeted either. They are capped at 64 MB in all; `mem`
  reports them as `spare_tails`.

## 3. The budget's cost: restart distance (VERIFIED, `raw/ab-sw*.jsonl`)

The host's `--budget BYTES` at 1,024 / 512 / 256 / 128 MB on the 1,000-page documents: typing on
pages 0, 300, 600, 999 in turn, 8 keystrokes each. One round each, so the p95s (4 keystrokes of 32)
are noisy.

Engines:
- `trim`: the side table and trim, with the old rungs;
- `rung0`: plus the first rung (every page checkpoint);
- `nest`: the final engine.

`trim` and `rung0` ran interleaved at load 10–27. `nest` ran later, **concurrently with the gates
(load 18–28)**: its latencies are high across the board, including at budgets where nothing was
dropped (plain-1000 at 1 GB).

| budget | doc | engine | peak RSS MB | checkpoints at the end | edited p50 / p95 ms |
|---|---|---|---|---|---|
| 1,024 | full-1000 | trim | 1,391 | 2,123 | 50.4 / 471 |
| | | rung0 | 1,456 | 553 | 32.7 / 155 |
| | | nest | 1,492 | 1,282 | 58.4 / 152 |
| | plain-1000 | trim | 588 | 1,895 | 13.9 / 21.8 |
| | | rung0 | 587 | 1,912 | 13.9 / 22.8 |
| | | nest | 615 | 2,333 | 28.8 / 42.5 |
| 512 | full-1000 | trim | 861 | 1,227 | 33.0 / 112 |
| | | rung0 | 982 | 1,177 | 27.3 / 41.9 |
| | | nest | 997 | 670 | 58.8 / 205 |
| | plain-1000 | trim | 588 | | 18.4 / 23.3 |
| | | rung0 | 592 | | 18.7 / 24.7 |
| | | nest | 618 | | 27.8 / 55.7 |
| 256 | full-1000 | trim | 574 | 1,181 | 30.9 / 125 |
| | | rung0 | 647 | 443 | 35.7 / 111 |
| | | nest | 629 | 711 | 40.1 / 267 |
| | plain-1000 | trim | 468 | 838 | 23.6 / 54.0 |
| | | rung0 | 514 | 734 | 19.0 / 55.0 |
| | | nest | 519 | 1,129 | 29.4 / 48.3 |
| 128 | full-1000 | trim | 432 | 397 | 35.8 / 111 |
| | | rung0 | 475 | 330 | 36.0 / 119 |
| | | nest | 447 | 365 | 52.4 / 157 |
| | plain-1000 | trim | 332 | 648 | 15.6 / 82.1 |
| | | rung0 | 360 | 466 | 22.4 / 53.7 |
| | | nest | 368 | 416 | 27.3 / 137 |

**What it shows.**
- The peak follows the budget: full-1000 at 128 MB peaks at 432–475 MB.
- plain-1000 never reaches 512 MB of logs, so budgets of 512 MB and up cost it nothing.
- The p95 grows below 256 MB on plain-1000. That is the first keystroke on a new page: it restarts
  up to a log-spaced gap before the edit, and later keystrokes there restart next to it.
- The p50 barely moves. Its rise at 256 MB and below is within these runs' noise (BELIEF).

**The default stays 1 GB (DESIGN §5.2).** A smaller default is the owner's call: 256 MB would keep
full-1000 under 0.65 GB at a p95 cost of roughly +0–100 ms on a jump to a far page (BELIEF, from
the noisy p95s above).

## 4. Latency before and after (VERIFIED, `raw/ab-ab1.jsonl`, `raw/ab-ab2.jsonl`)

Production builds, the host's defaults (keep-warm on, 1 GB budget), through the socket, pages 0 /
30% / 60% / last, 8 keystrokes each, 300 ms apart, engines interleaved per document:
- `base`: before (`6ddce18af`);
- `trim`: the side table and trim (`beb7b89d1`);
- `rung0`: plus the first retention rung (`593263746`).

Edited page p50 / p95 in ms over all keystrokes of the rounds, and peak RSS:

| doc | base | after | peak RSS base → after |
|---|---|---|---|
| plain-10 (ab1, 2 rounds) | 8.9 / 13.8 | 9.2 / 14.9 | 201 → 132 MB |
| full-10 (ab2, 3 rounds) | 19.0 / 35.3 | 20.1 / 41.5 | 371 → 153 MB |
| plain-120 (ab2, 3 rounds) | 9.4 / 12.4 | 9.7 / 13.4 | 682 → 176 MB |
| full-120 (ab2, 3 rounds) | 20.9 / 35.1 | 19.5 / 25.2 | 2,596 → 312 MB |
| plain-1000 (ab1, 2 rounds) | 15.4 / 194 | 17.8 / 25.4 | 5,536 → 595 MB |
| full-1000 (ab1, 2 rounds) | killed at 12 GB, twice | 40.2 / 138 | > 12 GB → 1,381 MB |

**plain-1000 after the review** (`raw/dbg3-summary.jsonl`, `raw/p4mem-dbg.tgz`). The review's 6
interleaved rounds put this branch's p95 at 49.3 ms against base's 30.2 ms. This rerun compares base
with the final engine `e545086f4` (tail reuse, deferred trim, the jump fix): 3 interleaved rounds at
load 22–33, the same keystrokes, with `FLASHTEX_INCR_DEBUG=1`. Medians per round:

| | base | final |
|---|---|---|
| edited page p50 / p95, pooled, ms | 24.2 / 38.1 | 23.0 / 36.3 |
| `restore` stage | 8.2–8.5 ms | 7.1–7.6 ms |
| of which: output tails | 1.6–1.9 ms | 0.7–0.8 ms |
| of which: undo | 0.61–0.62 ms | 0.80–0.89 ms |
| peak RSS | 7.3 GB | 0.66 GB |

ab2 also ran `notrim` (`rung0` with `FLASHTEX_NO_TRIM=1`):
- full-120 19.0 / 27.3 ms, 323 MB;
- plain-120 10.1 / 17.9 ms, 181 MB;
- full-10 20.2 / 41.2 ms, 172 MB.

The trim costs no measurable latency.

**What it shows.** Within the noise of a shared machine the edited page is as fast as before on
10–120 pages: p50s differ by −1.4 to +2.4 ms.

plain-1000 is not covered by this table's claim. ab1's 2 rounds gave a p95 of 194 → 25 ms, but the
review's 6 rounds found the branch slower at p95 (30.2 → 49.3 ms): the output-tail buffers below.
The table under it compares the final engine with base.

ab1's full-120 `trim` p95 (618 ms) came from 1–1.2 s stalls in one round with no restart or
memory cause (the restart was the page itself; the logs were 88 MB). base had 330–420 ms stalls
in both of its rounds. ab2 repeated full-120 three times, with `trim`'s code in `rung0` and
`notrim`: p95 25–27 ms against base 35 ms.

**Mac confirmation** (`raw/mac-summary.jsonl`; M5 Pro, load 3; 120 pages only, as briefed):

| doc | before (`m0`) | after (`m2`) |
|---|---|---|
| plain-120 | 494 MB, 7.0 / 10.0 ms | 207 MB, 6.5 / 9.8 ms |
| full-120 | 2,362 MB, 12.6 / 18.4 ms | 336 MB, 12.2 / 17.6 ms |

(`phys_footprint` lifetime maximum; edited page p50 / p95.)

## 5. The memory gate

`tools/incr-bench/mem_gate.sh [--build]` types on plain/full-120 and plain/full-1000 and fails when a
host's peak RSS passes its limit:

| doc | limit | measured |
|---|---|---|
| plain-120 | 0.6 GB | 0.18–0.20 GB |
| full-120 | 0.8 GB | 0.31–0.35 GB |
| plain-1000 | 1.2 GB | 0.59–0.62 GB |
| full-1000 | 1.8 GB | 1.38–1.50 GB |

Any host above 6 GB is killed. The nightly workflow runs it on the NixOS runner (`engine-memory`).
`mkeng.sh` now honours `CARGO_TARGET_DIR`, and the incr-bench scripts use `#!/usr/bin/env bash`:
NixOS has no `/bin/bash`, so `mkeng.sh` could not build a format there before.

## 6. Gates

The engine gates ran again after the review, on the NixOS PC at `e545086f4`, the final engine
code, with `tools/incr-bench/gates.sh` (J=6, at most six hosts at once, load 20–40;
`raw/gates-pc.tgz`). `scripts/gate.sh pr` ran on the Mac at the same commit (`raw/gate-pr-mac.txt`).
The first run, at `e0c81d691` before the review, gave the same counts for A, budget, C, D and book.

| gate | result |
|---|---|
| **span (new)**: `dlspan.py`, every glyph's source span after each edit (a letter or twelve words, accumulated) against a from-scratch host; plain-10 and full-10 × 3 seeds × 15 edits, plain-120 × 2 × 12, full-100 × 10 | **124 edits, 15,810,159 glyphs, 0 wrong**. The same check at `e0c81d691`, before the jump fix: plain-10 seed 1 had 930 glyphs on a wrong line and 2,925 at a wrong column (`raw/span-e0c81d691-plain-10.jsonl`), exit 1 |
| soundness A: 50 letters per document, plus reverts; 83 fixtures, plain-120, full-100 | **8,500 compiles, 0 mismatches** (768 converged; 10 logs differ in accounting only) |
| soundness under a 4 MB budget (new, `sound-budget`): 20 letters per document, plus reverts; the same documents; retention runs on every compile | **3,400 compiles, 0 mismatches** (342 converged) |
| the 4 MB budget with interleaved edits of every kind (new, `sound-budget-d`, the review's variant): fixtures, refs-30/120, full-100 | **948 verified + 156 interrupted, 0 mismatches** (60 converged) |
| soundness C: 20 structural edits | **1,966 compiles, 0 mismatches** |
| soundness D: 12 interleaved (preempted) edits | **1,409 verified + 223 interrupted, 0 mismatches** |
| soundness on book.tex: 8 letters and 4 sentences, plus reverts | **24 compiles, 0 mismatches** (all converged) |
| P-T1 / P-T2, 83 fixtures | **83/83 / 83/83** |
| lockstep | **1,145/1,145**; 1 case differs in accounting only, which does not gate |
| trip, etrip, drift | pass |
| display-list positions | **83/83** exact (230 pages, 118,899 glyphs) |
| cargo tests: incremental, host_incremental, display_list_host, intrinsics, lib | pass |
| `scripts/gate.sh pr` (Mac): rustfmt, clippy, tests, licence boundary, parity self-tests, fixture baseline | **passed** |

**After merging p4-finish-2 (`31aea85c7`)** the gates ran again, at `08ca78ece` (J=6;
`raw/gates-pc-merged.tgz`). The first run, at the merge commit `b5657ba49`, found **24 mismatches
in soundness A** (beamer-fragile under load). Two of the frames were titled from the previous
fragile frame's `.vrb`, and a font subset was 226 bytes short; soundness.py still exited 0.

- **Reproduction.** With a timed checkpoint every 0.2 ms (`--timed 0.0002`) it fails on 36 of 50
  compiles. That holds on p4-finish-2 itself (`31aea85c7`), so it predates this lane.
- **Cause.** A checkpoint taken inside a fragile frame has `main.vrb` open for output. Restoring it
  reopens the file at its length then and writes on after the bytes on disk. But the old run had
  truncated and rewritten the `.vrb` for later frames, so those bytes are another frame's.
- **Fix** (`5f0c79c68`). A file a run opens for output more than once is volatile
  (`system::volatile_output`). `restart_point` walks back past checkpoints that have such a file
  open, and `restore` refuses them.
- **Gates.** `sound-timed` (every fixture, 10 edits plus reverts, a checkpoint every 0.2 ms) now
  covers this. soundness.py exits 1 on any mismatch, and each run's records start afresh.

| gate at `08ca78ece` | result |
|---|---|
| span | 124 edits, 15,810,159 glyphs, **0 wrong** |
| sound-timed (new) | **1,660 compiles, 0 mismatches** |
| soundness A | **8,500 compiles, 0 mismatches** |
| soundness C | **1,966 compiles, 0 mismatches** |
| P-T1 / P-T2 | 83/83 / 83/83 |
| lockstep | 1,204/1,204 |
| trip | pass |
| `scripts/gate.sh pr` (Mac, `08ca78ece`) | passed |

**A rewritten file that is closed at the checkpoint** (focused re-review of #1300 at `44039a26e`).
The check above looked only at files open for output at the checkpoint.

- **The hole.** A temporary file can be written and closed before a checkpoint, then read back after
  it. The reviewer's `vol-closed` fixture (now `tools/incr-bench/genvol.py`) does exactly that.
  - It failed on 36 of 40 compiles with `--timed 0.0002` and on 4 of 20 with default checkpoints.
  - #1313 (`404305bdc`, per-checkpoint output opens) alone also failed it, 36 of 40.
- **The rule.** One rule now serves both the restart point's walk-back and the restores' refusal,
  on #1313's per-checkpoint output opens (`OPENS`, `rec.opens`). It is `rewritten_since` in
  `checkpoint.rs`: a file opened for output before the checkpoint and again after it, whether it
  is open or closed there.
  - The walk-back (`Globals::restorable`, `incr::Session::restart_point`) runs first.
  - A refusal by `restore` is the last resort: the compile then runs in full.
- **The `sound-vol` gate** runs `vol-open` and `vol-closed`, 20 edits plus reverts each, timed and
  default (`raw/gates-pc-proto.tgz`; prototype on a merge of #1313's head, `a6e7facaa`). Every
  compile was incremental:

  | fixture | timed | default |
  |---|---|---|
  | vol-open | **0/40** | **0/40** |
  | vol-closed | **0/40** (23 converged) | **0/40** |

- **Other gates at the same prototype.** All with 0 mismatches or 0 wrong:
  - span: 15,810,159 glyphs;
  - sound-timed: 1,660 compiles;
  - A: 8,500 compiles;
  - C: 1,966 compiles.

  Also: P-T1/P-T2 83/83, lockstep 1,204/1,204, trip pass.
- **No full recompile came from the rule.** Of 12,286 recorded compiles, none ran in full because
  of it. The cold ones were preamble edits and a failed run's missing PDF, as before.

The span check does not cover S₀ reopen spans (an S₀ carries none, by design) or preempted
compiles; the soundness sweeps' PDF, log and aux comparisons cannot see spans at all.

**The owner's book.tex** (1,072 pages; `raw/book-book-*.jsonl`): typing on pages 5, 130, 540 and
1,000, 6 keystrokes each. The final engine peaked at **1.02 GB**. The base engine passed the 10 GB
limit within 10 s, before its first keystroke, and was killed. The latencies of that run (p50 53 ms)
were taken at load 50 and are not quoted as a result.

## 7. What remains

1. **A quiet latency run.** The PC's load stayed above 10 all day, so the latency evidence is
   interleaved runs at equal load (§4): parity on 10–120 pages, and on plain-1000 for the final
   engine. The nested retention steps change only runs over the budget (full-1000, book.tex), and
   those were measured only under load. The side table's restore cost (+0.2–0.5 ms of undo per
   restore on plain-1000) remains.
2. **full-1000 sits at the edge of the target** (1.38–1.50 GB). The undo logs fill their 1 GB
   budget while a restart's detached run and the new run coexist. A smaller default budget (256 MB:
   0.63 GB peak) is the owner's call (§3).
3. **The budget does not count the host records** (up to 41 MB) or the slab's free blocks (up to
   17 MB).

## Reproducing

```
# NixOS PC (scripts/): build.sh; mkeng.sh NAME (NAME and NAME-ms, the mem-stats host)
bash scripts/run.sh TAG ENGINE plain-10 full-10 plain-120 full-120 plain-1000 full-1000
ROUNDS=2 KEYS=8 bash scripts/ab.sh TAG "ENGINE_A ENGINE_B" DOC...
KEYS=8 bash scripts/sweep.sh "ENGINE..." full-1000 plain-1000
python3 tools/incr-bench/mem_table.py raw/*.jsonl
tools/incr-bench/mem_gate.sh --build
```

`raw/p4mem-perkey.tgz` holds every keystroke's record (mode, restart page, convergence,
latencies, `mem`) and each host's stderr (the `malloc_trim` times).
