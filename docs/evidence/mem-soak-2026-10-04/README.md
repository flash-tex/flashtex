# MEMORY-SAFETY: a long editing session against the resident host (2026-10-04)

Lane **MEMORY-SAFETY** (mac-claude-a, mac-m1max-a). The owner asked for zero memory leaks. This
part soaks the resident engine host (`flashtex-host --socket`, crates/flashtex-engine): 600 to
1,200 keystrokes through the socket, as the app sends them, and checks that the host's memory
plateaus. DESIGN.md §5.2 (the budget drives retention), §1.2. Branch `agent/mac-claude-a/mem-soak`
from `origin/main` `f2012b591`.

**Labels.** VERIFIED means measured here; the run and its file are named. BELIEF means an inference
that was not measured.

**Machine.** M1 Max, 32 GB, macOS 26.3.1, TeX Live 2026 (MacTeX as the TeX tree only). The machine
was shared with other lanes throughout: load average 100 to 190, 6.8 of 8 GB of swap in use.
**Memory figures do not depend on load. Latency figures do, and none are quoted as results.**

## 1. The driver: `tools/incr-bench/soak.py`

`soak.py ENGINE DOC` starts one host (FLASHTEX_MEMSTAT=1) on a private copy of the document. It
speaks display-list-v3 as the app does: `incremental` COMPILEs with `edits` and a `viewport`, each
sent after the previous DONE plus a gap. The keystrokes are an editor's:
- 16 places spread over the body's prose lines; a burst stays at one place 70 % of the time.
- Bursts of 1-6 letters typed one keystroke each, then backspaced (85 %).
- Phrases of 3-12 words pasted, so the paragraph reflows, then deleted (85 %).
- Every 100 keystrokes whatever is still inserted is taken back (an undo), followed by an idle
  pause.

So the document keeps returning to states it had before. The driver checks at the end that the
files hold exactly what it sent. Every DONE's `mem` is recorded, and the footprint is sampled every
second. The host is stopped by PID.

**Two measures.**
- **footprint**: macOS `phys_footprint`, what Activity Monitor shows.
- **heap**: malloc's bytes in use less the undo logs (`mem.malloc_in_use - mem.sealed_bytes`). The
  host now reports `malloc_in_use` (`memstat::malloc_in_use`: `malloc_zone_statistics` over every
  zone on macOS, `mallinfo2` on glibc). It counts Rust's allocations and the C libraries' (zlib,
  kpathsea) alike, whether they are resident or not.

**The footprint cannot decide a plateau on a loaded machine** (VERIFIED). It moves with the undo
logs: retention and convergence drop and add checkpoints, ±40 MB on plain-120. Under memory
pressure the system also compresses and swaps leaked pages out of it:
- full-1000 (`fix2`, `raw/summaries.jsonl`) showed a **236 MB footprint while holding 697 MB of undo
  logs**;
- plain-120's footprint *fell* from 200 to 135 MB over 1,200 edits while the malloc zone's
  allocated bytes stayed at 150-200 MB (`vmmap -summary`).

The heap measure has neither problem, and every conclusion below rests on it. The footprint is
tabulated as well.

## 2. Curves (VERIFIED)

Engines, all release builds with `malloc_in_use` (`16345c93e` cherry-picked):
- `m-base`: `origin/main`.
- `m-ponly`: the parent's fixes alone (`agent/mac-claude-a/mem-sanitizers` `b04e8fa00`: zlib
  stream ended on drop, kpathsea started lazily with a memo of found files, read-set signatures
  re-taken).
- `m-fix`: both, this branch merged with the parent's.

Heap and footprint in MB. Warm is the median of edits 61-120 (plain-120), 51-100 (full-1000) or 8-15
(infdesc). Slope is the least-squares slope over the second half, in MB per 100 edits.

| doc, edits | engine | heap: open, warm, end (max) | heap slope | footprint: open, warm peak, end | footprint slope |
|---|---|---|---|---|---|
| plain-120, 1,200 | m-base | 81, 110, **399** (408) | **+26.5** | 200, 265, 500 | +19.7 |
| | m-ponly | 65, 70, 102 (108) | +2.5 | 189, 235, 229 | +4.5 |
| | **m-fix** | 65, 69, **71** (79) | **-0.5** | 186, 227, 188 | -8.2 |
| full-1000, 1,000 | m-base | 208, 395, **1,154** (1,185) | **+74.2** | 746, 1,078, 1,000 | +38.6 |
| | **m-fix** | 193, 319, **342** (362) | +7.0 (see below) | 659, 1,070, 306 | -1.9 |
| Infinite Descent (copy), 150 | m-base | 514, 515, 531 (532) | +11.6 | 636, 1,814, 1,056 | +141 |
| | **m-fix** | 497, 500, **501** (503) | **+1.7** | 559, 1,779, 1,057 | +463 (swap) |

Files: `raw/plain-120-m-*.csv`, `raw/full-1000-m-*.csv`, `raw/infdesc-m-*.csv` (every 20th, 25th or
5th edit: footprint, logs, heap, checkpoints, correction entries), `raw/summaries.jsonl`.

**plain-120**: in every 300-edit window, `m-base` grows 22-32 MB per 100 edits, `m-ponly` 1-6, and
`m-fix` -1.2 to +3.3.

**full-1000 `m-fix`** has a heap of 266-362 MB. It follows the number of retained checkpoints
(3,076-4,794; about 0.06 MB each for host records and position corrections), and at equal
checkpoint counts it is flat:
- edits 300 / 600 / 900: 331 / 327 / 334 MB at 4,298 / 4,287 / 4,463 checkpoints;
- edits 350 / 700: 275 / 266 MB at 3,402 / 3,076.

The second-half slope of +7 is the checkpoint count rising from 3,076 to 4,794 at the end, not a
leak. The undo logs stay within the 1 GB budget throughout.

**Infinite Descent** (~/Documents/infdesc copied to a private directory; 580 pages; never edited or
built in place):
- 150 edits at 8 places in the appendices (proof-writing, miscellany, LaTeX).
- An earlier `fix1` run typed at 8 places in probability-theory and additional-topics (44 edits
  before it was stopped; `raw/summaries.jsonl`, `infdesc-late-fix1`).
- The engine has every package it needs; the full-1000 fallback was not needed.
- Its edits **never converge** (0 of 150): every keystroke re-typesets from the edit to the end,
  50-95 pages here and 113-745 pages for edits in the main chapters, 7-170 s per keystroke at this
  load. That is why it got 150 edits instead of 1,000 (open item 2).

## 3. Findings and fixes

**(a) Lost C allocations: zlib once per restore, kpathsea per lookup.** These are about 280 KB per
edit on plain-120 and the bulk of `m-base`'s growth (`m-base` against `m-ponly`). They were found
and fixed by the parent with macOS `leaks` (commit `92dead754` on
`agent/mac-claude-a/mem-sanitizers`). They are not this branch's change. The soak confirms the fix:
+26.5 → +2.5 MB per 100 edits on plain-120. The mem-stats Rust heap had stayed flat (94 → 111 MB
over 400 edits) while the footprint grew, which pointed at C memory.

**(b) Position corrections kept per convergence, for ever** (this branch, `7532e9cfc`). Every
converged compile pushed a `Reloc` onto every later checkpoint (`incr.rs`, `Session::reloc`), each
with its own copy of the object positions the new run wrote. The entries of checkpoints dropped
since were never removed, and a restore applied hundreds of entries in turn. Measured on
`origin/main`:

| doc | edits | entries at the end | Rust heap |
|---|---|---|---|
| plain-120 | 1,200 | 306,000 | `heap_engine` 25 → 62 MB (mem-stats build, `plain-120-base-ms-1200`); with the fix 25 → 30 MB (`plain-120-fix1-ms-1200`) |
| full-1000 | 1,000 | 2,239,000 | |
| | | | the remaining +2.5 MB per 100 edits of `m-ponly` |

The fix: each checkpoint holds one composed correction.
- A piecewise shift: each step splits at most one piece, and equal neighbours merge.
- Its twin for `pdf_save_offset` and `pdf_stream_length_offset`, which move only while positive.
- The override words, with the value written and the value after later moves.
- The map is pruned with `ck_pages`/`defpatch`, keeping pending-branch ids.

Applying the composed correction leaves exactly what applying the steps in order left:
- `reloc_composes_exactly` compares the composed form with the step-by-step code (kept under
  `cfg(test)`) on random states and steps. It catches both deliberate breakages tried: the
  positive-only threshold, and the raw value for unvisited objects.
- `reloc_stays_bounded`: 10,000 steps at 3 places leave at most 4 pieces.

After the fix the number of entries is at most the number of checkpoints (`m-fix`: 421 on
plain-120, 4,775 on full-1000).

**(c) The hook's list of checkpoints taken (`Layer::taken`) never shrank.** It is read only by the
accounting. It grew by about 12 entries per edit on plain-120 and 910 per edit on Infinite Descent:
157,000 entries after 150 edits. It is now pruned to the retained and pending checkpoints once it
holds twice as many. Infinite Descent `m-fix`: 7,800-10,900 entries against 7,136 checkpoints.

**Regression test**: `a_long_session_keeps_its_bookkeeping_bounded` (`tests/incremental.rs`).
- 60 edits through `iserve` on a 200-paragraph hyperref document, at two places, at least 40 of them
  converging; 4 compared byte for byte with from-scratch runs.
- It asserts that the corrections never outnumber the checkpoints, that `taken ≤ 2 × checkpoints +
  65`, and that `reloc_bytes` stays within twice its value at edit 20 plus 64 KB.
- **On `origin/main` it fails at edit 5** (VERIFIED: the engine sources swapped back); it passes
  here.

**Not leaks, with the reason** (VERIFIED from the `mem` parts over the runs):
- The page cache: 12.6 MB on plain-120, 102 MB on full-1000, constant.
- The terminal buffer, the slab, the read-set bookkeeping (3.18 MB), `texts`/`written`: constant.
- Host records, `record_lines` and `reloc_bytes` follow the number of retained checkpoints, which
  the budget bounds.
- The undo logs swing within the budget.

## 4. The guard: `tools/incr-bench/soak_gate.sh`

```
tools/incr-bench/soak_gate.sh --build      # INCR_BENCH_DIR defaults to $RUNNER_TEMP/incr-bench-soak
```

600 keystrokes on plain-120 (gap 50 ms, idle 5 s after each undo). It fails when:

| check | limit | origin/main (`gate2-m-base`) | fixed (`gate2-m-fix`) |
|---|---|---|---|
| heap slope over the second half | ≤ 8 MB per 100 edits | **30.3: fails** | 2.2 |
| heap end | ≤ warm (edits 31-60) + 30 MB | **240 vs 96 + 30: fails** | 73.5 vs 66.8 + 30 |
| footprint end (loose; what malloc does not see) | ≤ 1.5 × warm peak + 100 MB | 309 vs 492 | 168 vs 453 |

The run took 134 s (fixed) and 266 s (main) at load 150. `tools/incr-bench/gates.sh` runs it as
`soak`.

**Why these numbers.** The fixed engine's 300-edit windows range from -1.2 to +3.3 MB per 100 edits,
and main's from 22 to 32. 8 sits 2.4× above the first and 2.8× below the second. The heap end limit
leaves 4.5× the fixed engine's largest excursion (+6.7 MB) at 600 edits.

**It does not catch a leak of the size of (b) alone** (2.5-6 MB per 100 edits). The engine test
holds that one.

**On Linux** the same gate reads `mallinfo2` and VmRSS. It was not run on Linux in this lane
(BELIEF that the thresholds carry over: the heap measure does not depend on the allocator's
retention, and glibc's `malloc_trim` runs during the 5 s idles).

## 5. What remains

1. **Unbudgeted memory that scales with checkpoints**: host records and the composed corrections'
   override lists (`reloc_bytes`: 4-46 MB on full-1000 at 3,000-4,800 checkpoints). They are
   bounded by the checkpoints and the objects each page writes, not by the budget (P4-MEMORY §7
   item 3, unchanged).
2. **Infinite Descent never converges**: every keystroke re-typesets to the end of the book. This is
   latency, not memory, and it is outside this lane. BELIEF: the `.hnt`/`.sol` exercise files
   written throughout the body and read back in the appendix keep the states from matching.
3. The footprint on a quiet machine was not measured. A quiet-machine soak would show the curve the
   owner sees in Activity Monitor; the heap measure above is the decision.

## Reproducing

```
export INCR_BENCH_DIR=$(mktemp -d)  CARGO_BUILD_JOBS=3
cargo build --release --locked -p flashtex-engine -p flashtex-display-list
tools/incr-bench/mkeng.sh m-fix && python3 tools/incr-bench/mkdocs.py
python3 tools/incr-bench/soak.py m-fix plain-120 --edits 1200
python3 tools/incr-bench/soak.py m-fix full-1000 --edits 1000
cp -R ~/Documents/infdesc /private/copy/infdesc    # never in place
python3 tools/incr-bench/soak.py m-fix infdesc --src /private/copy/infdesc --main infdesc.tex \
    --files 'book/proof-writing/*.tex,book/miscellany/*.tex,book/latex/*.tex' --sites 8 \
    --edits 150 --revert-every 50
python3 tools/incr-bench/soak_curve.py $INCR_BENCH_DIR/soak/*.jsonl
```
