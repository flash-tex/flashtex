# MEM-BASELINE: the resident host's footprint on small documents (2026-10-09)

Lane **MEM-BASELINE** (mac-claude-a, mac-m1max-a), for the Commander. As measured on main
`135b465fe`, the new engine's resident host used 4–11× the old engine's memory on small documents
(blank 88 MB against 11 MB, art1 106 against 15, hw1 101 against 26), and Low Memory barely helped
on them. DESIGN.md §1.2 ("memory everywhere"; performance modes), §5.2.

**Labels.** VERIFIED means measured here, with the script and raw file named. BELIEF means
inferred, not measured.

**Machine.** mac-m1max-a (M1 Max, 10 cores, 32 GB, macOS 26.3.1). It was shared with other lanes,
two self-hosted CI runners and other sessions, at a load average of 50–360. Footprint figures do
not depend on load. Instruction counts depend on it a little: a slowed run takes more timed
checkpoints (D7), so single runs scatter by 1–3 %, which is why every instruction comparison here
is interleaved and repeated. Thread CPU time scattered by up to 30 % between rounds, so it is
reported but not relied on.

## Method (VERIFIED)

`scripts/mb.py ENGINE DOC MODE` (derived from the MEM-MODES lane's driver) does the following:

1. starts the release `flashtex-host --socket --profile MODE` on a fresh copy of the document, with
   `FLASHTEX_MEMSTAT=1` (engines built by `tools/incr-bench/mkeng.sh`);
2. opens the document (`dl3-keys --keys 0`);
3. types 8 letters 300 ms apart at each edit place (16 for the instruction runs);
4. idles 8 s, past every mode's keep-warm and idle trim;
5. records `footprint` and `vmmap`;
6. opens the document once more, unchanged.

Only one host runs at a time, and nothing else of this lane ran meanwhile. `scripts/ab.sh`
interleaves the engines per document and first warms each engine with one discarded open, so that
the font-map and ls-R caches of #1720/#1721 exist, as they do for a user. Without that warm-up, a
run that builds the caches peaks 20–40 MB higher. `scripts/s0ab.sh` measures a reopen: a first run
writes S₀ to an `--s0-cache` directory, and the measured run opens from it, as the app does when it
reopens a document.

The quantities, all in MB of `phys_footprint` (Activity Monitor's "Memory"):

- **steady**: `footprint -p` after the 8 s idle;
- **peak**: the lifetime maximum (`DONE.mem.rss_peak`);
- **open**: `footprint -p` right after the first open.

Instructions are the engine thread's (`DONE.stages`):

- **edited page**: `edited_instr_k`, for the page the key is on;
- **keystroke**: `instr_k`, every pass of the keystroke's compile.

The documents (`scripts/mkdocs.py`):

- **blank**: an article with one line;
- **art1** and **art4**: `gen.doc(1|4, False)`;
- **hw1**: `fixtures/real-world/hw1`;
- **beamer-default**: `fixtures/real-world/beamer-default`;
- **full-100**: `gen.doc(100, True)`;
- **infdesc**: a copy of `~/Documents/infdesc`, 580 pages. The original was never written.

The scripts hold this machine's paths (`/private/tmp/mb`, the worktree); change them to rerun.

## Where a blank document's footprint went (VERIFIED, main `afe56e2a2`, Balanced, after the idle)

From `raw/vmmap-blank-m2.txt`, the footprint was 57–61 MB:

| part | dirty MB | note |
|---|---|---|
| word space (`VM_ALLOCATE`) | 32 | by array (`mincore`): eqtb 4.8, hash 4.8, font_info 4.8, mem 3.3, trie 3.3, str_pool 3.2, dl_side 1.3, pdf_char_used 1.1 |
| malloc (`MALLOC_*`) | 25 | 8 MB in use; `MALLOC_SMALL (empty)` alone 7–11 MB |
| `__DATA` and the rest | 2 | |

`__LINKEDIT` (18.9 MB resident), `__TEXT` (10 MB) and `mapped file` (13.4 MB) are clean pages: they
are resident but not in the footprint. Targets 3 and 4 cover them.

## Target 1: why the idle trim does not reclaim, and what does (VERIFIED)

**C tests.** These are `scripts/c/rel.c`, `reus.c`, `reus2.c`, `vmc.c` and `mf.c`; the outputs are
in `raw/c-*.txt`.

- **Default allocator (xzone).** Allocate N blocks, touch them, free them, then call
  `malloc_zone_pressure_relief(NULL, 0)`. The footprint keeps all of the freed memory from 16 KB to
  4 MB, and 24.6 of 40 MB at 1 KB. The relief returns 0 bytes. After 20 rounds of allocating,
  touching and freeing the same set, the footprint at 16 KB is **twice** the set (126 MB for 64 MB).
- **Environment variables.**
  - `MallocSpaceEfficient=1` and `MallocLargeCache=0` return the pages (1–2 MB is left), at 2–5×
    the churn time.
  - `MallocDeferredReclaim=0`, `MallocAggressiveMadvise=1`, `MallocXzoneDefer{Small,Large}=0` and
    `MallocXzoneSegmentDeallocate=1` change nothing. #1722 found the same for `MallocNanoZone` and
    `MallocXzoneEnabled`.
- **On a mapping of our own.**
  - `MADV_FREE` and `MADV_DONTNEED` leave the footprint as it was.
  - `MADV_FREE_REUSABLE` drops it at once (0.014 ms/MB), and `MADV_FREE_REUSE` counts it again
    (0.03 ms/MB).
  - A fresh page costs 0.1 ms/MB to fault in, and `munmap` drops it at once.
  - `mach_vm_copy` costs 0.006 ms/MB against `memcpy`'s 0.11, and the footprint does not hold both
    copies.
- **`MADV_FREE_REUSABLE` in the host.** It did **not** take spares out of the footprint once their
  mapping had been cut (spares are split and joined), nor pages `mach_vm_copy` had put there. On
  art4 the spares were reported marked, yet 28 MB of them were still counted. So the idle trim
  unmaps the spares instead.

**What the freed memory is.** From `malloc_history -allEvents` with `MallocStackLogging=full`, on
blank (open plus 8 keystrokes; `scripts/ev.py`, `raw/malloc-events-blank.txt`): 150 MB was allocated
over 9 compiles, **132 MB of it in blocks of 64 KB to 4 MB**, all transient. By owner:

- `diff_branch_inner`: 28.9 MB;
- the convergence test's `rewound_until`: 23.5 MB;
- `ChunkDiff::table`: 15.9 MB, 4 MB each;
- the prepared restore: 12.5 MB;
- `iso::check`: 9.5 MB;
- `free_cells`: 9.5 MB;
- `Plan::build`: 5.9 MB.

xzone keeps their pages after each keystroke.

**Conclusion.** The trim cannot reclaim on the default allocator, whatever the zone. Low Memory's
`MallocSpaceEfficient` (#1722) does reclaim, at a CPU cost. PR A moves the large blocks out of
xzone in the other modes.

**How the design was chosen.** On art4, against main, over 3 rounds of 32 keystrokes. The variants are
earlier states of PR A, or one build with temporary environment switches (`xp`, not in the PR):

| variant | keystroke instructions | art4 steady | file |
|---|---|---|---|
| zeroed blocks always fresh (no clearing), growth by `mach_vm_copy`, spares 32 MB | +2.0 % | | `raw/instr-a9-zeroed-fresh.txt` |
| zeroed blocks cleared in spares, growth by `mach_vm_copy`, spares 64 MB | +1.7 % | 57 MB | `raw/instr-xp-vmcopy.txt` |
| zeroed blocks cleared in spares, growth by copy, spares 64 MB | +0.2 % | 82 MB (spares marked reusable, still counted) | `raw/instr-xp-novmcopy.txt` |
| **as PR A** (the above, spares unmapped at the trim) | **−0.3 %** | **56 MB** | `raw/instr-a-final.txt` |

## Target 2: TeX's arrays (VERIFIED unless marked)

The arrays already live in one anonymous mapping of 532 MB (`arena.rs`). An untouched page costs
nothing, so `texmf.cnf`'s maximum sizes cost address space only. What is resident is what a run
touched. By array, on blank in Balanced (`DONE.mem.res_*`):

| array | first open (from the format) | reopen (from S₀) | what it holds |
|---|---|---|---|
| eqtb | 4.8 | 4.8 | non-zero throughout: §222 sets every entry, `hash_extra`'s too, to `undefined_control_sequence`'s |
| hash | 4.8 | 0 | zeros: §222 clears all of it, and the format uses a few hundred KB |
| font_info | 4.8 | 1.4 | 1.4 MB of fonts; the rest is written with zeros |
| mem | 3.3 | 3.3 | non-zero |
| trie | 3.3 | 3.3 | non-zero (every language's patterns) |
| str_pool | 3.2 | 3.2 | non-zero |

- **Sizing the arrays to what the format uses** gains nothing: the unused capacity is never
  touched.
- **Zero pages (PR B1).** TeX's own initialisation writes zeros over all of `hash` and over
  `font_info`'s tail at every load of the format: the first open and every preamble edit. The idle
  trim gives such pages back.
- **File-backed pages (PR B2).** After S₀, eqtb, mem, trie and str_pool are mostly read. S₀ v5
  stores its pages page-aligned, and a reopen maps them copy-on-write: they are clean file pages,
  out of the footprint until written. The arena code reads and writes them as before (a write copies
  the page, and the checkpoint logs copy chunks as before), so the representation is unchanged.
- **The format itself** is read through a buffered reader into the word space. No copy of it is
  kept, and nothing maps it.

## Target 3: `__LINKEDIT` (VERIFIED)

The full `vmmap` listing (`raw/vmmap-full-blank-m2.txt`) shows that the 18.9 MB resident
`__LINKEDIT` is **"dyld shared cache combined `__LINKEDIT`"**: the system libraries' symbol data in
the shared cache, mapped into every process and shared between them.

- The host's own `__LINKEDIT` is 1.4 MB, of which 32 KB is resident.
- The host's own `__TEXT` is 4.5 MB resident of 6 MB; the rest of `__TEXT` belongs to system
  libraries.
- All of it is clean and not in the footprint.
- Nothing in the host symbolicates except the panic hook (`Backtrace::force_capture`), and only on
  a panic.

The release profile (`debug = false`, cargo's default `strip`) needs no change.

## Target 4: "the format mapping" (VERIFIED)

The 13.4 MB `mapped file` is not the format. It is the ls-R index (`lsr-*.idx`, 9.2 MB mapped, 5.8
resident) and the font-map cache (6.8 MB) of #1720/#1721. Both are clean file pages: they count in
RSS but not in the footprint, and the system drops them under pressure. No change was made.

## PR A results (VERIFIED)

Engines: main `aa9c6e8ae` (m3) against PR A on it (a10). Six documents in three modes over two
rounds (ranges), and infdesc in one round (`raw/summary.jsonl`, tags `fa-` and `fi-`;
`scripts/prtab.py FA`):

| doc | mode | steady: main → PR | peak: main → PR | open: main → PR | edited-page instr M | keystroke instr M |
|---|---|---|---|---|---|---|
| blank | low-memory | 38 → **38** | 66 → 66 | 38 → 38 | 28.0 → 28.5 (+1.7 %) | 111 → 112 (+1.1 %) |
| blank | balanced | 88–93 → **44–45** | 88–93 → 76 | 73–74 → 73–74 | 28.3 → 28.1 (-0.5 %) | 108 → 108 (+0.2 %) |
| blank | high-performance | 89 → **75–77** | 89 → 75–77 | 73–74 → 73 | 27.8 → 27.9 (+0.2 %) | 107 → 108 (+0.1 %) |
| art1 | low-memory | 43 → **43** | 72 → 72 | 43–44 → 44 | 117.5 → 112.2 (-4.6 %) | 360 → 352 (-2.2 %) |
| art1 | balanced | 108–129 → **53** | 108–130 → 88 | 84 → 83–84 | 114.2 → 112.7 (-1.3 %) | 345 → 343 (-0.7 %) |
| art1 | high-performance | 122 → **88** | 122 → 88 | 84 → 83 | 111.2 → 112.5 (+1.1 %) | 341 → 342 (+0.3 %) |
| art4 | low-memory | 44 → **44** | 75–84 → 73 | 44–55 → 45 | 116.5 → 116.2 (-0.2 %) | 527 → 524 (-0.5 %) |
| art4 | balanced | 127–137 → **55–56** | 127–137 → 90 | 86–87 → 85 | 116.6 → 115.7 (-0.7 %) | 512 → 509 (-0.5 %) |
| art4 | high-performance | 136–144 → **88–90** | 136–144 → 89–90 | 86 → 85–86 | 118.1 → 114.8 (-2.8 %) | 513 → 508 (-1.0 %) |
| hw1 | low-memory | 47–54 → **47** | 79–81 → 79–80 | 48–49 → 49 | 274.4 → 275.4 (+0.4 %) | 453 → 456 (+0.6 %) |
| hw1 | balanced | 102–103 → **61** | 102–103 → 98 | 96–97 → 94–95 | 269.1 → 269.8 (+0.3 %) | 442 → 443 (+0.1 %) |
| hw1 | high-performance | 99–101 → **97–98** | 99–101 → 98 | 96–97 → 94–95 | 275.8 → 271.9 (-1.4 %) | 451 → 444 (-1.5 %) |
| beamer-default | low-memory | 68–81 → **67–69** | 111–119 → 112–117 | 69–71 → 68–70 | 206.4 → 205.2 (-0.6 %) | 809 → 804 (-0.6 %) |
| beamer-default | balanced | 169–177 → **91–92** | 170–178 → 148–149 | 139 → 141–142 | 204.4 → 205.1 (+0.3 %) | 794 → 787 (-0.8 %) |
| beamer-default | high-performance | 169–176 → **147–149** | 169–176 → 147–149 | 138–140 → 140–142 | 204.1 → 204.7 (+0.3 %) | 789 → 793 (+0.4 %) |
| full-100 | low-memory | 83 → **84** | 122–123 → 123 | 87–88 → 88 | 79.7 → 80.0 (+0.4 %) | 568 → 568 (-0.0 %) |
| full-100 | balanced | 209–213 → **131–132** | 210–214 → 181–183 | 176–177 → 178–179 | 77.3 → 78.5 (+1.5 %) | 549 → 550 (+0.3 %) |
| full-100 | high-performance | 246–247 → **192–196** | 247 → 194–198 | 193 → 189–193 | 78.3 → 78.6 (+0.5 %) | 558 → 563 (+0.9 %) |
| doc | mode | steady: main → PR | peak: main → PR | open: main → PR | edited-page instr M | keystroke instr M |
|---|---|---|---|---|---|---|
| infdesc | balanced | 448 → **473** | 659 → 586 | 528 → 494 | 433.5 → 432.9 (-0.1 %) | 153324 → 152370 (-0.6 %) |
| infdesc | high-performance | 787 → **759** | 908 → 820 | 686 → 619 | 498.2 → 496.6 (-0.3 %) | 159678 → 159194 (-0.3 %) |

Keystroke instructions and CPU, 16 keystrokes per place in Balanced, 3 rounds interleaved
(`scripts/instr2.sh`, `scripts/cpucmp.py`, `raw/instr-a-final.txt`):

```
art1             cpu ms             521 ->          522 (+0.26 %)  rounds [521, 514, 565] / [534, 500, 522]
art1             cycles M         1,587 ->        1,592 (+0.33 %)  rounds [1587, 1579, 1663] / [1618, 1541, 1592]
art1             instr M          5,445 ->        5,438 (-0.13 %)  rounds [5478, 5445, 5439] / [5467, 5434, 5438]
art4             cpu ms           1,058 ->        1,058 (-0.06 %)  rounds [1628, 1058, 1017] / [1306, 1058, 1037]
art4             cycles M         3,228 ->        3,231 (+0.10 %)  rounds [4317, 3228, 3104] / [3713, 3231, 3181]
art4             instr M         10,894 ->       10,858 (-0.33 %)  rounds [11069, 10894, 10853] / [10945, 10858, 10821]
beamer-default   cpu ms           1,201 ->        1,100 (-8.45 %)  rounds [1513, 1118, 1201] / [1136, 1100, 1094]
beamer-default   cycles M         3,643 ->        3,373 (-7.42 %)  rounds [4280, 3426, 3643] / [3451, 3373, 3362]
beamer-default   instr M         12,578 ->       12,533 (-0.36 %)  rounds [12750, 12536, 12578] / [12587, 12529, 12533]
full-100         cpu ms           2,576 ->        2,539 (-1.43 %)  rounds [3209, 2560, 2576] / [3094, 2533, 2539]
full-100         cycles M         7,897 ->        7,778 (-1.50 %)  rounds [9101, 7856, 7897] / [8971, 7762, 7778]
full-100         instr M         26,755 ->       26,682 (-0.27 %)  rounds [26930, 26711, 26755] / [27042, 26682, 26673]
```

**Notes.**

- Main moved during the lane. Between `afe56e2a2` and `aa9c6e8ae`, blank's footprint after the
  first open rose from 44 MB to 73 MB, and its steady footprint from 57–61 MB to 88–93 MB (VERIFIED
  with this harness). PR A brings the steady figure back to 44–45 MB. The open figure is main's.
  BELIEF: the cause is main's S₀ written by a background thread from a copy of every touched chunk
  (`S0Image`).
- infdesc ran once. Earlier rounds of the same engine on it spread by 10–30 % (148–203 MB in Low
  Memory), so its steady row is not significant. Its peak fell in Balanced in all three pairs run
  (639/649/659 to 540/552/586 MB).
- Low Memory is unchanged by design: the host turns the mappings off under `MallocSpaceEfficient`.

## Outputs (VERIFIED)

After the same keystrokes, every run's PDF (without its `/ID`, an MD5 of the output path and time) and
`.aux` are byte-identical between main and the PRs: 124 files over the runs above
(`scripts/pdfcmp.py`, `raw/outputs-identical.txt`). The logs differ only in the work directory's
name, through the string-pool statistic and a line break. The tables for PRs B1 and B2 are in
`raw/tables-b1-b2.md`, and the reopen check for S₀ v5 (#B2: 83 fixtures reopened and edited, each
equal to a from-scratch run; 3 not reopened because their `.aux` changed) is in
`raw/reopen-check-all.txt`.

## Reproducing

```
python3 scripts/mkdocs.py                              # documents under /private/tmp/mb/src
INCR_BENCH_DIR=/private/tmp/mb/ib tools/incr-bench/mkeng.sh NAME   # per engine, from its checkout
bash scripts/ab.sh TAG "BASE PR" "low-memory balanced high-performance" "blank art1 art4 hw1 beamer-default full-100" 2
bash scripts/s0ab.sh TAG "BASE PR" "low-memory balanced high-performance" "blank art4"
bash scripts/instr2.sh OUTDIR "BASE PR" "art4 full-100" 3 && python3 scripts/cpucmp.py OUTDIR BASE PR
```
