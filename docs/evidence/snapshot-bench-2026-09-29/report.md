# Engine v2 §5.2: choosing the checkpoint mechanism (2026-09-29)

Lane P0-SNAPSHOT-BENCH. Branch `agent/kabir-claude/snapshot-bench`; benchmark at
`tools/snapshot-bench/`, raw output in [`raw/`](raw/) (`environment.txt`, one `.md` table
and one `.jsonl` per phase). Reproduce with `tools/snapshot-bench/run.sh`.

**Host:** mac-m5pro-kabir, Apple M5 Pro (10 P + 5 E cores), 24 GiB, Darwin 25.6.0,
**16 KB pages**, rustc 1.98.0, `--release`. Medians over 50 repetitions (timing phases) and
60 rounds (hot-loop phases), with the minimum and p90 beside them. 1-minute load average
4.2–6.0 across the bundle; see *Measurement conditions*.

## Recommendation

**Take (b), software 16 KB-chunk copy-on-write — but build it as a flat word space with a
dirty bitmap and an undo log, not as a chunk table of `Arc`s.** (a) and (c) are both ruled
out by measurement, and so is the `Arc`-chunk-table spelling of (b) at 200 MB.

| | snapshot ≤ 1 ms | standing barrier | per-checkpoint copies | memory the engine can see |
|---|---|---|---|---|
| (a) kernel `mach_vm_remap` | **yes** — 78 µs / 237 µs | **zero** | **12× the software cost** | **no** — invisible to the task ledger |
| (b1) `Arc::make_mut` table | 183 µs / **1.25 ms — fails** | +0.77…0.92 ns/op | 1.2× | yes |
| (b2) chunk table + bitmap | 189 µs / **1.21 ms — fails** | +0.73…0.94 ns/op | 1.2× | yes |
| **(b3) flat + bitmap + undo log** | **yes** — 2.3 µs / 7.5 µs | **+0.27…0.33 ns/op** | **1.0×** | yes |
| (c) full memcpy | **1.24 ms / 3.58 ms — fails** | zero | n/a | yes |

Two figures per cell are the 64 MB and 200 MB states. The decisive points:

1. **(c) is out.** A full copy of the arenas costs 1.240 ms at 64 MB and 3.584 ms at
   200 MB — it fails the 1 ms gate at every state size the design cares about. Pooling the
   destination buffer does not help (1.068 ms / 3.462 ms): the cost is the `memcpy`, not the
   allocation.

2. **(a)'s barrier is genuinely free, but its copies are not — and copies are what a
   checkpoint costs.** There is no instruction-level barrier; reads and writes are plain
   loads and stores, measured at or slightly below the `plain` baseline. But the first store
   to each page after a snapshot takes a kernel copy fault, and that fault costs **4.76 µs
   per 16 KB page in the real access pattern** against **0.41 µs for a software chunk
   copy** — a factor of 12. With a checkpoint per `\shipout` at 64 MB, (a) adds +10.12 ns
   per access where the undo log adds +0.88. A sequential fault probe puts (a)'s floor at
   1.6 µs/page, so even its best case is 3× the software copy.

3. **(a)'s memory cannot be measured from inside the process.** Eight
   `mach_vm_remap(copy=TRUE)` snapshots of a 64 MiB region, with every page of the live copy
   then rewritten, hold 512 MiB of distinct data — verified page by page, every snapshot
   still reading its own generation's value from a distinct mapping. `phys_footprint`,
   `internal`, `compressed` and `resident_size` move by **exactly zero**; the host's
   free-page count falls by **509 MiB**. §5.2 requires retention "within a configurable
   budget (default 1 GB)". Under (a) the engine would have to maintain its own dirty-page
   table to know what it holds — which is mechanism (b)'s bookkeeping without mechanism
   (b)'s control over the pages. Pinned by
   `mach::tests::kernel_cow_memory_is_invisible_to_the_task_ledger`.

4. **The `Arc`-chunk-table spelling of (b) fails the 1 ms gate at 200 MB**: 1.211 ms median
   and 1.385 ms p90 for (b2), 1.247 ms / 1.355 ms for (b1). The snapshot is a clone of the
   chunk table, which at 200 MB is 12,800 `Arc` clones — an atomic read-modify-write on a
   control block sitting in its own 16 KB allocation, so one cache miss per chunk. Measured
   directly: the `Arc` table clone costs **667 µs** at 200 MB where an equivalent id table
   plus a contiguous reference-count array costs **3.96 µs** — a factor of **168**.

5. **(b3) passes everything with room to spare.** Keeping the live state as one flat
   `Vec<u64>` and logging a chunk's previous contents before its first write since the
   checkpoint gives snapshot **2.3 µs / 7.5 µs** (130–160× under the gate), restore
   **73 µs / 166 µs** (the best of any mechanism), and a standing barrier of
   **+0.27…0.33 ns per access** on a bare replay loop, falling to **+0.01…0.29 ns** once
   there is realistic per-access work for the core to overlap it with — against a measured
   noise floor of ±0.01…0.13 ns/op.

(b3) beats (b1)/(b2) because a chunk table is paid for on the **read** path. Privatising a
chunk moves it, so every access goes through the table: two dependent loads instead of one,
paid by the 75% of accesses that are reads and never touch the barrier at all. Undo logging
does not move the live data, so reads are exactly `plain`'s single load and only writes are
barriered. Dropping the table roughly **thirds** the standing barrier.

### Conditions on the recommendation

- **The 3% gate cannot be closed until the v2 engine has a hot loop to measure.** What is
  settled here is the barrier's absolute cost; the gate is a ratio whose denominator does
  not exist yet. See *Does it meet the 3% gate?*: (b3) passes for any engine costing
  ≥ 10 ns per memory access on the pessimistic reading and ≥ 1.7 ns on the realistic one,
  while the chunk table needs ≥ 24–31 ns, which pdfTeX may or may not clear. **Re-run the
  barrier phase against the real engine before closing P4.**
- **Restore becomes O(chunks dirtied since the target checkpoint).** Shadow paging restores
  in O(1) by swapping the table; an undo log replays. Restoring to the newest checkpoint is
  73–166 µs, better than every alternative, but restoring to an old one walks every log in
  between, bounded above by a full-state copy (about 1 ms at 64 MB). That is once per edit,
  not once per page, so it is the right trade — but if §5.3's early-stop makes deep walks
  common, adjacent logs need compacting (merge, keeping the oldest version of each chunk).
- **Chunks must come from a slab, not `malloc`.** The 1,000-checkpoint run held 680.7 MiB
  where the chunk-version accounting says 549.3 MiB: **1.24× overhead** on 16 KB
  allocations.
- **All mutable arenas must share one flat word space** so a single bitmap covers them.
  That is what this benchmark's layout assumes and what §5.2 will need to state.
- **§5.2's retention default does not fit §5.2's own budget.** Dense-16 plus 4-per-octave
  over 1,000 checkpoints holds 680 MiB in the best case and 2.00–3.69 GiB at 200 MB. The
  1 GB budget has to drive the policy, not the other way round.
- **These numbers are specific to 16 KB pages.** On a 4 KB-page host (Intel, or Rosetta)
  kernel granularity and software chunk size diverge and the comparison changes.

## What was measured

One flat `u64` word space split into TeX's arenas, at four sizes. `mem` absorbs the
remainder, which is how one layout spans both §5.2's stated `mem` range and the
assignment's total-state points:

| name | total | chunks | `mem` | `eqtb` | `hash` | `save_stack` | `str_pool` | `font_info` |
|---|---|---|---|---|---|---|---|---|
| `mem768k` | 14.4 MiB | 921 | 768,528 w | 60,000 | 65,536 | 80,000 | 262,144 | 650,000 |
| `mem5M` | 46.7 MiB | 2,988 | 5,001,744 w | 〃 | 〃 | 〃 | 〃 | 〃 |
| `total64MB` | 64.0 MiB | 4,096 | 7,270,928 w | 〃 | 〃 | 〃 | 〃 | 〃 |
| `total200MB` | 200.0 MiB | 12,800 | 25,096,720 w | 〃 | 〃 | 〃 | 〃 | 〃 |

The hot loop's locality comes from DESIGN Appendix B.1 — **about 58 KB freed per
`\shipout`**, `mem` 768k words in use, 2.4 ms per body page. `get_node` takes nodes off the
rover free list, so a page's allocation churn is reused storage in a window that drifts
slowly forward, not fresh storage scattered over all of `mem`. The `tex-freelist` profile
models that: `mem` writes 98% inside a drifting 128 KB window or the recently built graph
behind it and 2% uniform (box registers, token lists, marks, inserts); `eqtb` writes 90% in
the parameter block; `save_stack` sequential around an oscillating group depth; `str_pool`
appended; `hash` and `font_info` read-only. **`uniform` is the pessimal control** — every
write uniform over its arena — and it is reported throughout, because the answer depends on
which is true.

Correctness is gated by 21 self-tests (`cargo test --release -p snapshot-bench`): every
mechanism must round-trip a snapshot, survive repeated restores from the same checkpoint
(§5.3 restarts while converging), keep two snapshots independent, and — for the kernel path
— leave the live range writable after a `VM_FLAGS_OVERWRITE` remap.

## Snapshot and restore

Median of 50, with min and p90 where they matter. **Bold** fails the 1 ms gate.

| state | mechanism | snapshot | restore |
|---|---|---|---|
| 14.4 MiB | kernel `mach_vm_remap` | 16.25 µs (min 13.54, p90 19.88) | 32.54 µs |
| | kernel `vm_copy` | 35.25 µs | — |
| | (b1) `Arc::make_mut` | 10.46 µs | 23.17 µs |
| | (b2) chunk + bitmap | 10.29 µs | 22.88 µs |
| | **(b3) flat + undo log** | **1.29 µs** | 34.04 µs |
| | (c) memcpy | 334.46 µs | 231.88 µs |
| 46.7 MiB | kernel `mach_vm_remap` | 59.21 µs | 110.33 µs |
| | kernel `vm_copy` | 57.33 µs | — |
| | (b1) `Arc::make_mut` | 107.00 µs | 202.54 µs |
| | (b2) chunk + bitmap | 110.88 µs | 171.62 µs |
| | **(b3) flat + undo log** | **2.08 µs** | 62.08 µs |
| | (c) memcpy | 922.58 µs | 765.83 µs |
| **64 MiB** | kernel `mach_vm_remap` | 77.71 µs (min 70.79, p90 104.88) | 140.08 µs |
| | kernel `vm_copy` | 62.04 µs (min 42.42, p90 88.88) | — |
| | (b1) `Arc::make_mut` | 183.25 µs (p90 239.71) | 310.04 µs |
| | (b2) chunk + bitmap | 188.92 µs (p90 246.83) | 278.08 µs |
| | **(b3) flat + undo log** | **2.29 µs** (p90 2.71) | **72.54 µs** |
| | (c) memcpy | **1.240 ms** (p90 1.429) | 1.066 ms |
| **200 MiB** | kernel `mach_vm_remap` | 236.67 µs (min 207.21, p90 283.42) | 485.83 µs |
| | kernel `vm_copy` | 125.71 µs (min 97.83, p90 208.58) | — |
| | (b1) `Arc::make_mut` | **1.247 ms** (min 754.04 µs, p90 1.355 ms) | 1.612 ms |
| | (b2) chunk + bitmap | **1.211 ms** (min 648.17 µs, p90 1.385 ms) | 1.576 ms |
| | **(b3) flat + undo log** | **7.54 µs** (p90 9.33) | **166.38 µs** |
| | (c) memcpy | **3.584 ms** (p90 3.727) | 3.384 ms |

`vm_copy` into a pre-allocated region is consistently cheaper than `mach_vm_remap` at the
larger sizes (126 µs vs 237 µs at 200 MB) and, at 3.5% of the equivalent `memcpy`, is
plainly not an eager copy either. (b3)'s snapshot is dropping the previous interval's log
and clearing the bitmap; a chained implementation seals the log instead of freeing it, which
leaves the bitmap clear alone — the observed minima of 0–42 ns are exactly that case.

### Why (b1)/(b2) miss the gate at 200 MB

| state | chunks | `Arc` table clone | id-table `memcpy` | + contiguous refcount pass |
|---|---|---|---|---|
| 14.4 MiB | 921 | 10.25 µs | 42 ns | 292 ns |
| 46.7 MiB | 2,988 | 99.46 µs | 166 ns | 875 ns |
| 64 MiB | 4,096 | 175.42 µs | 167 ns | 1.17 µs |
| 200 MiB | 12,800 | **666.67 µs** (p90 949.46 µs) | 750 ns | **3.96 µs** |

The snapshot cost is entirely per-chunk `Arc` refcount traffic, and it is 150–168× what the
same bookkeeping costs in a side table. If shadow paging were kept for other reasons, that
is the fix; (b3) sidesteps it by having no table to clone.

## The write barrier

All six backends live at once, the same page's op stream replayed through each inside every
round, median over 60 rounds, order reversed on odd rounds so drift applies to all equally.
**`memcpy` has no barrier at all, so its column is this experiment's noise floor.**
`every 8 pages` is §5.2's ~20 ms trigger at 2.4 ms per body page.

`total64MB`, `tex-freelist`, 1% touched/page (83,886 accesses/page), ns per access relative
to `plain`:

| checkpoint | work | kernel | (b1) `Arc` | (b2) chunk | **(b3) undo** | control |
|---|---|---|---|---|---|---|
| none | replay | −0.34 | +0.77 | +0.73 | **+0.30** | +0.01 |
| none | engine | +0.14 | +0.17 | +0.18 | **+0.01** | −0.02 |
| every page | replay | +9.13 | +2.12 | +2.04 | **+1.40** | −0.11 |
| every page | engine | +10.12 | +1.31 | +1.22 | **+0.88** | −0.16 |
| every 8 pages | replay | +2.24 | +1.55 | +1.46 | **+0.88** | −0.03 |
| every 8 pages | engine | +3.29 | +1.06 | +0.75 | **+0.59** | +0.09 |

`total200MB`, `tex-freelist`, 1% (262,144 accesses/page):

| checkpoint | work | kernel | (b1) `Arc` | (b2) chunk | **(b3) undo** | control |
|---|---|---|---|---|---|---|
| none | replay | −0.27 | +0.92 | +0.94 | **+0.32** | −0.00 |
| none | engine | +0.36 | +0.54 | +0.55 | **+0.29** | +0.13 |
| every page | replay | +8.24 | +1.88 | +1.72 | **+1.11** | −0.03 |
| every page | engine | +8.75 | +1.13 | +0.98 | **+0.69** | −0.11 |
| every 8 pages | engine | +2.50 | +1.06 | +1.05 | **+0.66** | −0.02 |

The standing barrier is stable across all four state sizes and both locality profiles:
kernel −0.27…−0.34 (zero, plus a bounds-check advantage from raw pointers over `Vec`),
(b1) +0.77…+0.92, (b2) +0.73…+0.94, **(b3) +0.27…+0.33**, control ±0.01.

Dividing the copy cost out: at 64 MB / `tex-freelist` / 1% a page dirties 176 chunks, so the
`every page` minus `none` delta gives **4.76 µs per kernel copy fault**, **0.50 µs per (b2)
chunk copy** and **0.41 µs per (b3) chunk copy**.

### Does it meet the 3% gate?

The gate is a ratio and its denominator — the v2 engine's own cost per memory access — does
not exist yet, so the honest form of the answer is the break-even point:

| mechanism | barrier, replay (no ILP) | 3% needs ≥ | barrier, engine work | 3% needs ≥ |
|---|---|---|---|---|
| (a) kernel | ~0 | any | +0.14…0.36 | 4.7–12 ns/access |
| (b1) `Arc::make_mut` | +0.77…0.92 | 26–31 ns/access | +0.17…0.54 | 5.7–18 ns/access |
| (b2) chunk + bitmap | +0.73…0.94 | 24–31 ns/access | +0.18…0.55 | 6.0–18 ns/access |
| **(b3) flat + undo log** | **+0.27…0.33** | **9.0–11 ns/access** | **+0.01…0.29** | **0.3–9.7 ns/access** |

For scale, B.1's 2.4 ms body page divided by this model's accesses per page implies
9.2 ns/access at 200 MB and 28.6 ns at 64 MB. **(b3) clears 3% on both readings at both
sizes; the chunk-table variants clear it on the optimistic reading only.** The `replay`
column is a true upper bound — a bare index-replay loop gives the core no arithmetic to
overlap the barrier's bitmap load with, and a real engine does.

If the gate is read as covering the copies as well, then at the `\shipout` cadence nothing
passes 3% ((b3) is +0.88 ns/op, 3.1% of a 2.4 ms 64 MB page) and at the ~20 ms cadence (b3)
reaches +0.59 ns/op, 2.1%. That is the sharper argument for preferring the timer trigger
over per-`\shipout` checkpointing on fast body pages.

## Memory per checkpoint

Both copy-on-write mechanisms copy at 16 KB on this host, so memory per checkpoint is the
dirty-chunk count times 16 KB either way — and the retention run confirms it: at 64 MB with
realistic locality the kernel mechanism held 676.0 MiB against the software mechanisms'
680.7 MiB, agreeing to 0.7%. Chunks, not words, are what a checkpoint costs, and the gap
between the two middle columns below is the whole question.

Budget as % of state words **touched** (read or written) per page — the reading consistent
with B.1:

| state | locality | touched | distinct words written | chunks dirty | memory/checkpoint |
|---|---|---|---|---|---|
| 64 MiB | tex-freelist | 1% | 15,574 (0.19%) | 176 / 4,096 (4.3%) | **2.8 MiB** |
| 64 MiB | tex-freelist | 5% | 41,994 (0.50%) | 454 / 4,096 (11.1%) | **7.1 MiB** |
| 64 MiB | uniform | 1% | 20,628 (0.25%) | 2,332 / 4,096 (56.9%) | 36.4 MiB |
| 64 MiB | uniform | 5% | 101,643 (1.21%) | 3,565 / 4,096 (87.0%) | 55.7 MiB |
| 200 MiB | tex-freelist | 1% | 32,477 (0.12%) | 339 / 12,800 (2.6%) | **5.3 MiB** |
| 200 MiB | tex-freelist | 5% | 83,148 (0.32%) | 1,213 / 12,800 (9.5%) | **18.9 MiB** |
| 200 MiB | uniform | 1% | 64,032 (0.24%) | 7,496 / 12,800 (58.6%) | 117.1 MiB |
| 200 MiB | uniform | 5% | 315,312 (1.20%) | 12,181 / 12,800 (95.2%) | 190.3 MiB |

Budget forced to 1% and 5% of state words **written** per page, with the op budget
calibrated by bisection until the distinct write set lands on the target:

| state | locality | dirty target | achieved | chunks dirty | memory/checkpoint |
|---|---|---|---|---|---|
| 64 MiB | tex-freelist | 1% | 1.00% (402,653 ops/page) | 1,240 / 4,096 (30.3%) | **19.4 MiB** |
| 64 MiB | tex-freelist | 5% | 2.80% — arena-capped | 3,466 / 4,096 (84.6%) | 54.2 MiB |
| 64 MiB | uniform | 1% | 1.08% | 3,560 / 4,096 (86.9%) | 55.6 MiB |
| 64 MiB | uniform | 5% | 5.02% | 3,584 / 4,096 (87.5%) | 56.0 MiB |

Note the gap between the two framings. 1% of state words *dirtied* per page is 672 KB
written on a 64 MB state, 11× B.1's measured 58 KB freed per `\shipout`. The assignment's
"1% and 5% dirty" is therefore a pessimistic scenario rather than the expected one; B.1's
own numbers land nearer 0.2% dirty words and **2.8 MiB per checkpoint**.

**Locality is the whole result.** 1% of words dirtied is 4.3% of chunks under free-list
locality and 56.9% under uniform scatter — a 13× difference in checkpoint memory from the
same word count. Sensitivity to the uniform fraction of `mem` writes (64 MB, 1% touched):

| uniform share of `mem` writes | 0% | 2% | 5% | 10% | 20% | 50% | 100% |
|---|---|---|---|---|---|---|---|
| chunks dirty | 105 | 177 | 285 | 456 | 757 | 1,524 | 2,345 |
| memory/checkpoint | 1.6 MiB | 2.8 MiB | 4.5 MiB | 7.1 MiB | 11.8 MiB | 23.8 MiB | 36.6 MiB |

Chunk copy-on-write pays only while long-lived scattered writes stay a few per cent of
`mem` traffic. Past roughly 20% scatter a checkpoint costs more than a fifth of the state
and the advantage over a full copy erodes. Anything in the engine that writes uniformly
across `mem` every page — a compacting collector, a rebuilt global index — would invalidate
this recommendation, so §5.2 should record write scatter as an invariant to watch.

## 1,000 checkpoints under log-spaced retention

TeXpresso-style decimation, dense 16 plus 4 per octave, retaining **40 of 1,000**. Software
figures measured by a counting global allocator; the kernel figure from the host's
free-memory drop, since its pages are invisible to the task ledger.

| state | locality | touched | mechanism | total | per checkpoint |
|---|---|---|---|---|---|
| 64 MiB | tex-freelist | 1% | chunk CoW | **680.7 MiB** | 17.0 MiB |
| | | | kernel | 676.0 MiB | 16.9 MiB |
| | | | memcpy | 2.50 GiB | 64.0 MiB |
| 64 MiB | tex-freelist | 5% | chunk CoW | 1.15 GiB | 29.4 MiB |
| | | | kernel | 1.24 GiB | 31.8 MiB |
| 64 MiB | uniform | 1% | chunk CoW | 1.89 GiB | 48.3 MiB |
| | | | kernel | 1.89 GiB | 48.5 MiB |
| 64 MiB | uniform | 5% | chunk CoW | 2.20 GiB | 56.4 MiB |
| 200 MiB | tex-freelist | 1% | chunk CoW | 2.00 GiB | 51.3 MiB |
| | | | kernel | 1.87 GiB | 47.9 MiB |
| | | | memcpy | 7.81 GiB | 200.0 MiB |
| 200 MiB | tex-freelist | 5% | chunk CoW | 3.69 GiB | 94.6 MiB |
| | | | kernel | 3.34 GiB | 85.5 MiB |
| 200 MiB | uniform | 1% | chunk CoW | 6.34 GiB | 162.4 MiB |

Copy-on-write beats a full copy by 3.8× at 64 MB and 3.9× at 200 MB with realistic
locality, and the chunk-version accounting tracks the allocator figure at 1.24× (the
`malloc` overhead noted above). The kernel and software figures agree to within 1–10%
wherever the host-wide metric is clean, which is the expected answer: identical 16 KB
granularity means identical memory.

**Only the first row fits §5.2's default 1 GB budget.** The budget must bound the policy
rather than the reverse: at 64 MB with realistic locality that is roughly 58 retained
checkpoints, at 200 MB about 20.

A caveat on the kernel rows: the host-wide free-page metric also sees other processes
releasing memory, so individual rows can read implausibly low — two rows in
[`raw/retention.md`](raw/retention.md) read 521 MiB and 0 B for that reason. The software
measurement is exact and per-process and is the one to quote; the kernel figure is
corroboration only.

## macOS pitfalls

Verified on Darwin 25.6.0 unless marked otherwise.

1. **Kernel copy-on-write memory is invisible to the task's own accounting** (measured; see
   point 3 above). The pages are real — the host loses them — but `phys_footprint` does not
   move, so neither jetsam accounting nor an in-process retention budget can see them.
2. **The arenas cannot live in `Vec`/`malloc` memory** (API constraint, not measured).
   `mach_vm_remap` and `vm_copy` act on whole pages of a VM region; `malloc` sub-allocates
   pages and keeps its metadata in them. Mechanism (a) therefore forces every mutable engine
   arena through a dedicated page-aligned allocator — a design condition, not an
   implementation detail.
3. **`mach_vm_remap`'s protection arguments are in/out and their handling has varied across
   XNU versions.** Initialising both `cur_protection` and `max_protection` to
   `VM_PROT_READ|VM_PROT_WRITE` works here: the kernel returns `cur 0x3` and `max 0x7` for
   both the snapshot remap and the `VM_FLAGS_OVERWRITE` restore, and the live range stays
   writable — asserted by `tests::kernel_state_is_writable_after_restore`, because a restore
   that silently dropped `VM_PROT_WRITE` would fault forever. Note `max` comes back with
   `VM_PROT_EXECUTE` set, broader than requested.
4. **`VM_FLAGS_OVERWRITE` restore keeps addresses stable.** The live base is unchanged, so
   raw indices and pointers into the arenas survive a restore. This is (a)'s one real
   structural advantage — and a TeX port addresses its arenas by index, so (b) gives it up
   cheaply.
5. **`copy=TRUE` marks the source as needing a copy too**, so the fault lands on the next
   write to the *live* state, not to the snapshot. That is (a)'s barrier, and it is why its
   cost appears as a burst immediately after each checkpoint rather than as a per-access
   tax.
6. **`vm_copy` needs a pre-allocated, page-aligned destination**, and is not eager: 126 µs
   for 200 MB against 3.46 ms for the equivalent `memcpy`.
7. **`fork()` checkpoints, as TeXpresso uses, are not available here** (Apple's documented
   constraint; not measured). After `fork()` without `exec`, only async-signal-safe
   operations are legal, and Core Foundation, libdispatch and the Objective-C runtime make
   it undefined. The engine host shares its address space with the Swift/Core Graphics
   preview renderer (§6.2, B.3), so those frameworks are loaded. `mach_vm_remap` needs no
   fork, which is why (a) was benchmarked in that form.
8. **16 KB pages make kernel and software granularity identical**, so (a) has no
   granularity advantage on this host — and none of these numbers transfer to a 4 KB-page
   host.
9. **`Arc::make_mut` has no non-atomic fast path**: it proves uniqueness with a
   `compare_exchange` on the strong count on every call, hit or miss. Its cost is
   structural, not a tuning problem.
10. **`malloc` is a poor fit for 16 KB chunks**: 1.24× measured overhead.

## Measurement conditions and limitations

- **This host is shared with other build lanes.** The bundle ran at load 4.2–6.0; every
  timing row records its 1-minute load average, and `run.sh` refuses to measure above 3.0
  unless told otherwise. An earlier pass at load 19 inflated the copy-fault figure about 2×
  and pushed (b2)'s 200 MB snapshot from 994 µs to 1.21 ms — which is why a
  minimum-of-repetitions column sits beside every median. The *ranking* of mechanisms was
  identical in every pass.
- **The workload is synthetic.** Its locality is anchored on B.1's measured 58 KB per
  `\shipout`, but it models TeX's allocator, not TeX. The `uniform` control brackets it from
  the pessimal side and the sensitivity sweep shows exactly where the conclusion would flip.
- **The hot loop is an index-replay loop**, so its `ns/op` is below a real engine's and the
  `replay` ratios over-state relative overhead. That is why the barrier is reported as
  absolute ns per access with a break-even table, not as one percentage.
- **(b3) is implemented with a single level of undo log** — it restores to the newest
  checkpoint only. Its memory is unchanged by chaining (the same dirty chunks at the same
  granularity), but a multi-level restore walks several logs, so the retention figures above
  are measured on the shadow-paging backends, which is conservative for (b3).
- **The interleaved barrier phase keeps six backends resident**, which raises absolute
  `ns/op` through cache and TLB pressure. It is the drift-controlled comparison; the
  single-pass `hotloop` phase is the realistic-footprint one. Both are in `raw/`.

## What P4 should do next

1. Implement (b3) in the engine: one flat arena word space, a 16 KB dirty bitmap, a sealed
   undo log per retained checkpoint, chunks from a slab.
2. Re-run the barrier phase against the real hot loop, to close the 3% gate with a real
   denominator.
3. Set §5.2's retention policy from the 1 GB budget rather than fixing dense-16 /
   4-per-octave.
4. Record "scattered `mem` writes stay under a few per cent of `mem` traffic" as an engine
   invariant, with the sensitivity table above as its justification.
