# MEM-FOOTPRINT: where the resident host's memory goes, and what cuts it (2026-10-04)

Lane **MEM-FOOTPRINT** (mac-claude-a, mac-m1max-a). Owner goal (2026-10-04): the new engine uses
less memory than the old one and is faster; Infinite Descent host ≤ 0.5 GB steady (from
1.3–1.6 GB, `docs/evidence/infdesc-app-2026-10-03/`), a 100-page article ≤ 150 MB. DESIGN.md
§1.2, §5.2 (arena, checkpoints, retention budget), §7. Lane P4-MEMORY
(`docs/evidence/p4-memory-2026-09-30/`) before it; MEMORY-SAFETY (#1493, #1505) fixes leaks,
this lane the steady footprint.

**Labels.** VERIFIED: measured here, the command and raw file named. BELIEF: inferred, not measured.

**Machine.** mac-m1max-a (M1 Max, 10 cores, 32 GB), shared with about seven other sessions; load
5–380 during these runs. Memory figures do not depend on load. Wall-clock latencies do, so each
latency comparison also gives the engine thread's **instructions** for the edited page
(`DONE.stages.edited_instr_k`, the PMU counter), which do not.

## Method

`scripts/run.py` (copied from `/tmp/mfp`, the lane's scratch area): starts the production
`flashtex-host --socket` (release, `tools/incr-bench/mkeng.sh`) on a fresh copy of the document
with `FLASHTEX_MEMSTAT=1`, opens it with `dl3-keys --keys 0` (compiles until nothing changes),
types with `dl3-keys` at each place in turn (a letter inserted and deleted, 300 ms apart), idles
8 s, opens once more (unchanged; its `DONE.mem` is the steady record), then takes `vmmap -summary`
and `footprint`. One host at a time.

- **full-100**: `tools/incr-bench/gen.py`'s full-100 (101 pages: article, hyperref, siunitx,
  cleveref), 8 keystrokes on pages 10, 50, 90.
- **long-deck**: `fixtures/beamer-v3/long-deck` (118 slides, Madrid), 8 keystrokes in frames 19
  (slide 38) and 47 (slide 94), `dl3-keys --line`.
- **infdesc**: a copy of `~/Documents/infdesc` (the original is never written), 580 pages
  (external tools off, as main ships), 6 keystrokes in `propositional-logic.tex`,
  `divisibility.tex` and `series-sums.tex`.

Quantities: `phys_footprint` (vmmap's "Physical footprint": dirty plus compressed memory, what
Activity Monitor shows) now and its lifetime peak; the C allocator's bytes in use and held
(`malloc_in_use`/`malloc_held`, new in `DONE.mem`); the undo logs (`sealed_bytes`) and checkpoint
count. Attribution by owner: `MallocStackLogging=lite` and `malloc_history -allBySize` on the live
host, summed by the innermost engine frame (`scripts/mh.py`; `raw/malloc-history-*.txt`).

**A caveat on "steady".** Under the machine's memory pressure the system compresses idle pages, so
the steady footprint of a large host moves with what else runs: infdesc measured 0.93–1.5 GB
steady at the same code. The lifetime peak and `malloc_in_use` do not move with it; the tables
give all three.

## Phase 1: the breakdown (VERIFIED, main `4fa1b3df1` plus the accounting of PR 1)

| doc | footprint steady | peak | malloc in use / held | undo logs | checkpoints |
|---|---|---|---|---|---|
| full-100 | 268–316 MB | 292–316 MB | 186–200 / 270–291 MB | 86–88 MB | 516–578 |
| long-deck | 237–269 MB | 262–270 MB | 161 / 233–241 MB | 70 MB | 537–566 |
| infdesc | 0.93–1.6 GB | 1.8–1.9 GB | 1,329–1,407 / 1,973–2,056 MB | 802–822 MB | 6,770–7,240 |

(`raw/table-phase1.txt`, runs `p1`, `mh`, `A-base`.)

**By owner, live heap** (`malloc_history`; the MSL build of the same host):

| owner | full-100 | infdesc |
|---|---|---|
| undo logs (`Arena::checkpoint`: sealed deltas and words) | 97 MB | **915 MB** |
| virtual-font packets copied on write (`Shared<vfpacket::State>`) | 0 | **265 MB** (5.4 M allocations) |
| checkpoint host records (`capture_ext`, `redo_to_remapped`'s copies) | 4 MB | 61 MB |
| kpathsea's `ls-R` database (`kpathsea_init_db`, 1.03 M allocations) | 37 MB | 37 MB |
| the font map (`pdftex.map`: `avl_do_entry`, `read_field`) | 24 MB | 24 MB |
| the display-list page cache (`Page::encode`) | 11.5 MB | 32 MB |
| zlib deflate states (`deflateInit2`) | 8 MB (120) | 6 MB (90) |
| the prepared restore (`rewound_until`) | 4 MB | 9 MB |
| PDF object tree (`avl_put_obj`) | 0.4 MB | 9 MB |
| definition sites (`diag::dg_def`) | 1.7 MB | 6.8 MB |
| arena bookkeeping (`Plan::build`) | 3.2 MB | 3.2 MB |

**The word space** (`mmap`ed anonymous memory, `VM_ALLOCATE`): 532 MB reserved, 36 MB (full-100)
to 56 MB (infdesc) ever written, 23–39 MB resident. TeX's fixed-size tables are already committed
on touch, so their worst-case sizes cost address space only. Resident by array (`res_*`, 16 KB
pages): `mem` 6–12 MB, the display list's side table `dl_side` 4–7.5 MB, `str_pool` 4.6–7 MB,
`eqtb`, `hash`, `font_info` about 5 MB each, `trie` 3.5 MB. The slab of open-log chunks: 5 MB
(full-100) to 16–19 MB (infdesc) resident; it never unmaps.

**Elsewhere.** The format (15.4 MB) is read and undumped into the word space; no copy of it stays
(the largest live `fs::read` is 0.9–3.5 MB). Thread stacks: 537 MB virtual, under 0.1 MB resident.
`malloc_zone_pressure_relief` (tried as the macOS counterpart of P4-MEMORY's `malloc_trim`)
returned 0 bytes in 0.00 ms; the allocator already marks free pages reusable (vmmap: of
infdesc's 1.6 GB resident small-zone pages, 700 MB are dirty), so the gap between held and in use
is mostly not in the footprint (BELIEF from vmmap's columns; not changed).

**What it says.**
1. The undo logs are the largest item everywhere: 30 % of full-100, 60–70 % of infdesc. infdesc
   keeps 12 checkpoints per page (segment checkpoints every 0.5 ms of engine time on pages of
   about 190 ms), and nothing is thinned: the 1 GB budget is never reached.
2. One copy-on-write bug: 265 MB on infdesc (PR 1).
3. 61 MB per host is fixed cost from the TeX Live side (the `ls-R` database and the font map),
   37 % of what full-100's 150 MB target allows.
4. The page cache, the records, the prepared restore and the rest are 10–60 MB each.

## PR 1 (`memfp-vfpacket`): virtual-font packets shared across checkpoints (VERIFIED)

`vfpacket.c`'s state is one `Shared` (copy-on-write) value in pdfTeX's C state, and every packet
read moves its cursor (`packet_byte`, `start_packet`), so the first virtual-font character shipped
after each checkpoint copied every packet of every virtual font. The packets are now shared per
font inside the state, so the copy takes the cursor and two reference counts.

- The encoding is the plain nested vectors' (persisted S₀ unchanged): test
  `encoding_is_the_plain_vectors`.
- The convergence test compares the state by the encodings' equality, as before, without encoding
  shared packets: test `same_as_is_encoding_equality`.

Same main, same keystrokes, engines one after the other (`raw/table-pr1.txt`):

| doc | engine | footprint peak | malloc in use | edited page p50 / p95 (instructions) |
|---|---|---|---|---|
| infdesc | base | 1.8 GB | 1,339 MB | 370 / 585 M |
| | PR 1 | **1.5 GB** | **1,106 MB** (−233 MB) | 506 / 591 M |
| full-100 | base | 292 MB | 186 MB | 102 / 142 M |
| | PR 1 | 296 MB | 186 MB | 102 / 144 M |
| long-deck | base | 270 MB | 161 MB | 384 / 388 M |
| | PR 1 | 267 MB | 160 MB | 382 / 387 M |

full-100 and long-deck use no virtual fonts: no change, as expected. On infdesc the restart pages
and gaps are the same per keystroke (`raw/keys-pr1-infdesc.txt`); the edited page takes 287 M
instructions instead of 327 M on `series-sums.tex` (keys 2–6: no copy of the packets), the same on
`divisibility.tex`, and more on `propositional-logic.tex` only because the idle-time prepared
restore had not run there (`restore` 190 M instructions per key against 43 M; the machine's load
rose from 8 to 180 during that run, and the preparation is cut short by the next request).
Wall-clock times of these runs are in the raw files and are not a comparison.

## Reproducing

```
# build (production) and install as engine NAME under /tmp/mfp/ib, with its format
CARGO_BUILD_JOBS=4 cargo build --release -p flashtex-engine -p flashtex-display-list
INCR_BENCH_DIR=/tmp/mfp/ib tools/incr-bench/mkeng.sh NAME
# documents in /tmp/mfp/docs/{full-100,long-deck,infdesc}; then
bash scripts/ab.sh TAG "ENGINE..."        # the three documents, one host at a time
python3 scripts/table.py TAG...           # the rows above
MallocStackLogging=lite PROBE='malloc_history {pid} -allBySize > {out}.mh' python3 scripts/run.py ...
python3 scripts/mh.py OUT.mh              # live heap by owner
```
