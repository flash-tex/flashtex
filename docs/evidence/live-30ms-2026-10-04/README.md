# LIVE-30MS: keystroke to updated pixels (2026-10-04)

Lane **LIVE-30MS** (mac-claude-a, mac-m1max-a). The owner's requirement (2026-10-04): an edit to *Infinite
Descent* (592 pages) takes 200+ ms to reach the live preview. The target is **keystroke → updated pixels
≤ 30 ms p95, roughly O(1) in page count**. DESIGN.md §1.2 (latency targets), §5.1–§5.3 (S₀, checkpoints,
convergence), §5.6–§5.7, §6.2 (preview).

**Machine.** M1 Max (10 cores, 32 GB), on AC, shared with other sessions: **load1 20–135** throughout.
**No wall time here is a reference.** The instruction counts (DONE's `*_instr_k`, the engine thread's
fixed counters) do not move with load. Every number is from **main `d07191179`** (before #1493, the memory
lane; none of #1493's per-keystroke lookup changes are in it) plus this lane's instrumentation.

## Phase 1: where a keystroke's time goes (VERIFIED)

Tools: `scripts/keyrun.py` (one `flashtex-host --socket` per document, `dl3-keys` phases, every keystroke's
DONE kept), `scripts/perkey.py` (per keystroke, with `queue_by`), `scripts/table.py`, `scripts/interval.py`,
the app's `FLASHTEX_V3_BENCH`. The phases:

- `iso` is one keystroke after the previous `DONE` plus 300 ms.
- `typing` is `--overlap --gap-ms 100`: the next key 100 ms after the edited page arrived, while the
  previous compile's background work goes on.
- `--interval-ms N` (new) types on a clock, as a user does. A key's latency runs to the watched page's
  first `PAGE` from its own compile or a later one.

New in DONE's `stages`: **`queue_by`**, which splits the wait of a request that arrived while the engine
thread was busy by what the thread was doing then (`busy.rs`: restore, typeset, test, jump, prepare, done…).

### Infinite Descent (a copy; 580 pages without the tools' index; edit in `book/number-theory/divisibility.tex`, page 253)

| stage (p50) | iso | typing | instructions |
|---|---:|---:|---:|
| edited page, client (p50/p95) | 101/134 ms | 147/210 ms | |
| queue (the previous compile) | 0 | 1/75 ms (p50/p95): the **convergence jump**, 60–130 ms, non-preemptible | |
| find (S₀ key check 8.7 ms, what changed 3.5 ms) | 14 ms | 19 ms | |
| restore | 9 ms | 22 ms (18–47 ms; the prepared restore never completes) | 40 M (iso) |
| re-typeset to the edited page's shipout | ~80 ms | ~80 ms | **540–670 M** |
| display list | 1.7 ms | 1.6 ms | |
| convergence test (after the page; not interruptible at the viewport page) | 33 ms | 29 ms | 330–590 M |
| background after DONE | 2–40 s CPU (the middle of the book never converges: 328 pages, 191 G instr per key) | | |

**The TeX floor.** One cold pass is 298 G instructions for 580 pages, about **514 M per page**
(`raw/macro-profile-infdesc-top80.tsv`). Where it goes:

- **47 %** in mdframed's TikZ-framed theorem boxes (`thm@framedpostwork` / `fb@put@frame`).
- pgfmath parsing throughout.
- `\@outputpage` 10 %.

Re-typesetting one page of this book is therefore ~80 ms of TeX at any restart granularity. 30 ms on it
needs macro replay (P6-HYPEROPT, MACRO-REPLAY.md: these are its next targets) or a comparable raw-speed
cut, not incremental work.

### Synthetic documents (gen.py; host's edited page p50/p95 ms; letter edit)

| doc | mid iso | mid typing | start (page 1) iso | start typing | restore instr, typing (M) |
|---|---|---|---|---|---:|
| plain-10 | 12/21 | 19/65 | 10/18 | 11/17 | 9 |
| plain-100 | 9/12 | 12/26 | 11/27 | 18/48 | 13 |
| plain-600 | 13/47 | 39/95 | 15/23 | 104/216 | 30 |
| plain-1000 | 15/25 | 28–40/54–81 | 14/68 | 90/231 | 44 |
| full-10 | 23/57 | 34/87 | 22/104 | 60/130 | 11 |
| full-100 | 16/23 | 22/46 | 15/17 | 54/111 | 13 |
| full-600 | 40/86 | 47–93/74–153 | 25/57 | 91/129 | 42 |
| full-1000 | 28/38 | 34–43/70 | 22/29 | 34/46 | 32 |

`raw/base/` (main engine, mid), `raw/busy/` (+ `queue_by`), `raw/start/` (page 1 and mid typing);
`scripts/table.py DIR` makes this table.

### The app (plain-1000, typing at page 1 every 150 ms; `raw/app/plain1000-150.json`)

- **key → commit p50/p95 153/745 ms.** The host's first page takes 131 ms: every page after the edit is
  old future to rewind.
- **Starvation.** A compile that newer keys preempt before its edited page ships paints nothing, so one
  commit covers 4–5 keys, and that is the p95.
- App stages: edit hook → sent 9.3 ms, raster → main thread 13.7 ms; decode 0.1, prepare 0.04,
  raster 1.9, commit 0.5 ms.
- *Infinite Descent* in the app gave 0 samples: the bench never scrolled to the edited page. The bench now
  reveals the caret's page before typing (`EngineV3Bench`).

### Not O(1) in page count (VERIFIED: `raw/infdesc-incr-debug.txt`, instruction counts)

1. **The restore's rewind** applies every undo log after the restart point. On *Infinite Descent* that is
   3,200 logs (14–20 ms). The prepared restore (`prepare_next`) does the same work ahead of time, and fast
   typing interrupts it.
2. **The convergence test** rewinds the whole old future to get the old checkpoint's values of its
   **1.6k candidate chunks**, walking 4.6 M log entries (23–49 ms).
3. **The convergence jump** (`redo_to_remapped`) is non-preemptible. It repeats that rewind
   (`diff_branch_all`) and rebuilds the read-set from every event. It also splices the old PDF tail after
   the new run's and remaps every later checkpoint's record.
4. The display list, socket, decode, raster and commit are O(1) per page.

## Phase 2: results (VERIFIED, mac-m1max-a, load1 100–310)

**Metric.** Wall time measured only the load here. Each keystroke's **engine latency** is the engine
thread's cycles from its COMPILE arriving to the first page of the compile that painted it (DONE's
`arrival_mark_kc`, `first_page_mark_kc`), at 3.2 GHz. The work on its critical path, whatever the load;
it leaves out the socket and the client (< 1 ms). The typist is on a clock (`dl3-keys --interval-ms`),
which on a loaded machine types faster *relative to the engine* than a user would: a harsher test, the
same for every engine. Runs are interleaved by document (`/tmp/l30/ab.sh`); `scripts/pool.py` pools.

### Synthetic documents (plain-100, full-100, plain-1000 × middle/page 1 × 100/150 ms; 2 rounds each, 540 keys per engine)

| run | engine | engine p50/p95/p99 ms | keys never painted |
|---|---|---|---|
| ab6 | main | 12.4/39.3/64.0 | 2 |
| ab6 | + D (every compile held to its page: first version) | 14.3/49.7/70.3 | 4 |
| ab6 | + C | 14.0/39.8/65.2 | 2 |
| ab6 | + B | 12.8/38.1/59.2 | 4 |
| ab6 | + A | **12.1/28.2/49.0** | 0 |
| ab7 | main | 15.0/53.2/81.1 | 2 |
| ab7 | + D (adaptive: held only after a starved compile) | 15.2/49.6/72.7 | 4 |
| ab7 | + C | 14.7/42.2/67.6 | 0 |
| ab7 | + B | 13.8/37.5/59.0 | 6 |
| ab7 | + A | **13.5/37.0/57.0** | 2 |

(`raw/phase2/ab6`, `raw/phase2/ab7`.) Run-to-run spread is large (main's p95 39 vs 53), so only
deltas within a run are meant to be read. Deterministic effects (`scripts/stagecmp.py`, instructions):

- B: the restore after an abandoned run on plain-1000 drops from 44 to 11–14 M p50.
- A: keeps about half the old chunks a comparison needs (e.g. 29k kept / 33k rewound).

### Infinite Descent (580 pages; dl3-keys --edit, a key on a clock; `raw/phase2/infab`)

| row | main: engine p50/p95 ms, keys painted | D(full)+C+B+A: engine p50/p95 ms, keys painted |
|---|---|---|
| middle of the book, every 150 ms | **none painted (0/30)**: every compile preempted before its page | 148/359, 26/30 |
| middle, every 300 ms | 269/736, 25/30 | 168/348, 30/30 |
| late in the book, every 150 ms | 240/612, 25/30 | 130/174, 30/30 |

The restore drops from 136 to 43 M instructions. The edited page itself stays 520–600 M instructions
(≈ 165–190 ms at 3.2 GHz): the TeX floor of this book (mdframed + TikZ frames), for macro replay (P6).

### A per-chunk log index: tried, worse, not landed

The next step for A was to rewind each missing chunk through the logs that hold it only (a per-chunk
index of the sealed logs, branch `agent/mac-claude-a/live30-index-attempt`). It was exact (unit test
and verify mode) but **slower**: plain-1000's engine p95 went 27 → 100 ms (`raw/phase2/ab8`, engine
"a6"). Hot chunks (the page builder's state, counters) are in every page's log, so a missing hot chunk's
list is as long as the old future, and a hash look-up per entry costs more than the plain walk's bit
test. Bounding the comparison further needs keyframes (a materialised old state every N pages), not an
index.

### Soundness (sweeps A, C, D; FLASHTEX_VERIFY_JUMP, FLASHTEX_VERIFY_PREPARED, FLASHTEX_VERIFY_OLDCACHE on, each aborting on a disagreement; none did)

| build | sweep A | sweep C | sweep D (interleaved) |
|---|---|---|---|
| D(full) + C | 2112 compiles, **0** mismatches, 367 converged | 580, **0**, 26 | 804 (712 verified, 92 interrupted), **0**, 107 |
| D(full) + C + B + A | 2112, **0**, 367 | 580, **0**, 26 | 804 (712/92), **0**, 108 |

Sweep C's error is the harness's known "0 trials" on beamer-sans-operators-professional. The lookup
sweep needs #1507's tooling (not on main).

## Phase 2 (one PR each, in the Commander's order)

| PR | change | status |
|---|---|---|
| live30-measure | `queue_by`, engine-cycle marks, `dl3-keys --interval-ms`, these scripts, the bench's caret reveal | #1533 |
| D live30-antistarve | after a starved compile, the next is not preempted before its changed page; a superseded compile still sends that page | #1541 |
| C live30-preempt | no jump while newer work waits; the jump's comparison first and stoppable; an interruptible viewport test | #1542 |
| B live30-reattach | a reattach leaves the restore to its target prepared | #1543 |
| A live30-oldstate | old checkpoints' chunk values kept between comparisons | this PR |
| E live30-app | the fast path's splice counts only the text near the edit | #1538 |
| E live30-s0key | S₀'s key runs the preamble's lookups again only when a directory changed since they last held | separate PR |
