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

**Gates** (`raw/gate-pr-1.txt`, `scripts/gate.sh pr` at `6735a5d31`, load 230–330): rustfmt, the
licence boundary, the parity self-tests and the parity fixtures' baseline pass. Clippy found one
`unnecessary_sort_by` in the new accounting (fixed in the next commit; `cargo clippy -p
flashtex-engine --all-targets -D warnings` then passes). Of the engine's tests one failed:
`host_lifetime::a_once_host_exits_when_no_connection_comes_in_time` (the debug host did not exit
within its 300 s limit at load 300); rerun alone, both `host_lifetime` tests pass.

## PR 2 (`memfp-segments`): far from the cursor only page checkpoints stay (VERIFIED)

Retention (`incr::thin`, DESIGN.md §5.2: "dense near the cursor and viewport, log-spaced
elsewhere") only ran once the undo logs passed the 1 GB budget, which none of these documents
reaches, so every segment checkpoint (one per 0.5 ms of engine time between pages) was kept on
every page. Its first step, which merges the segment checkpoints more than `DENSE` (16) pages from
the cursor into their page's checkpoint, now runs on every compile and every 32 checkpoints of a
run, within the budget too. The budget's further steps (log-spaced page checkpoints) are as they
were; the default budget is unchanged.

Why it pays (offline, `scripts/merge.py` on the sealed logs dumped from a host, a
measurement-only build): adjacent logs hold the same words again, because TeX rewrites the same
nodes, so merging every 5 logs of full-100 leaves 49 of 84 MB, and the 621 page logs of infdesc
merged two by two leave 239 of 382 MB.

PR 1 against PR 1 + PR 2 (engines `vf` and `seg`, both on main `4fa1b3df1` with the accounting;
`seg` is this PR's code; `raw/table-pr2.txt`):

| doc | checkpoints | undo logs | malloc in use | footprint steady / peak |
|---|---|---|---|---|
| infdesc | 6,849 → **621** | 810 → **382 MB** | 1,106 → **532 MB** | 0.98 → 0.68 GB / 1.5 → 1.3 GB |
| full-100 | 541 → 124 | 86 → 49 MB | 186 → 139 MB | 296 → 247 MB / 296 → 274 MB |
| long-deck | 535 → 131 | 69 → 43 MB | 160 → 126 MB | 267 → 210 MB / 267 → 234 MB |

The live heap of infdesc after it (`raw/malloc-history-infdesc-pr2.txt`): logs 391 MB, kpathsea
37, page cache 32, font map 24, prepared restore 9, PDF objects 9, definition sites 7, zlib 6,
host records 7.

**The cost: the first keystroke at a new place.** Typing at one place costs the same; the first
keystroke more than 16 pages from the last edit restarts at its page's start instead of the
segment before the edit (`raw/keys-pr2-*.txt`, instructions of the edited page):

| doc, place | first keystroke | the next ones |
|---|---|---|
| full-100 page 10 (near the open's cursor) | 112 → 104 M | 101–102 → 100–102 M |
| full-100 page 50 | 105 → 116 M | 93–99 → 94–99 M |
| full-100 page 89 | 144 → 183 M | 143–144 → 139–141 M |
| infdesc `divisibility.tex` (page 250) | 582 → 725 M | 454–591 → 484–506 M |
| infdesc `series-sums.tex` (page 384) | 342 → 425 M | 286–288 → 320–337 M |
| infdesc `propositional-logic.tex` (page 49) | 515 → 417 M | 505–511 → 354–388 M |

At 3.2 GHz and about 1 instruction per cycle (BELIEF) the first keystroke at a far place costs
up to 12 ms more on full-100 and up to 45 ms more on infdesc; LIVE-30MS's metric (keystrokes at
one place) does not move. Two places move either way at the same restart page, gap and convergence page:
`series-sums` keys 2–6 take 30–50 M instructions more than PR 1's, `propositional-logic` keys 2–6
120–150 M fewer. Not explained; not measured further.

**Soundness** (this PR's commit `b5636a2ac`, `scripts/sound.sh`; `raw/soundness/`): the rule
needs documents over 16 pages, so the sweeps ran on the long ones only (the fixtures never trigger
it):

| sweep | documents | compiles | mismatches |
|---|---|---|---|
| A: 30 letters + reverts | plain-120, full-100 | 120 (114 converged) | **0** |
| C: 12 structural edits + reverts | refs-120, full-100 | 46 | **0** |
| D: 8 interleaved edits | refs-120, full-100 | 29 verified + 6 interrupted | **0** |

The over-budget form of the same step was already gated by `sound-budget` (a 4 MB budget, every
run thinned; `tools/incr-bench/gates.sh`). `scripts/gate.sh pr` at `b5636a2ac` passed
(`raw/gate-pr-2.txt`; the fixture scorer reported one dead worker, as on PR 1's run).

## PR 3 (`memfp-logcodec`): sealed logs pack each half-word in 0–4 bytes (VERIFIED)

A sealed log's words are pre-images of TeX's memory words: two 32-bit halves (`link`/`info`, a
`scaled`, character and font codes), mostly small. Each half is now stored in 0, 1, 2 or 4 bytes
(zero, then the smallest sign-extended width), with a 2-bit tag; a word's two tags are one nibble,
and a delta's nibbles precede its bytes. Measured offline on the dumped logs (`scripts/tags.py`):
0.50 of the bytes on infdesc, 0.58 on full-100. (Rejected: a zero-word mask alone saved 9 %
(`scripts/zero.py`); zlib reaches about 0.2–0.3 but needs a whole log decompressed for one word.)

- A restore (`Delta::apply_under`) decodes only the words it still needs; the others' sizes come
  from their nibbles. A merge copies a lone delta's bytes and repacks only where two deltas meet.
  `or_from` (rare) rewrites the log it changes.
- Tests: every arena test now writes words of every packed width (`shaped`);
  `packed_words_round_trip` checks the codec and `apply_under` with partly done masks.
- Microbenchmarks (release, `restore_cost`, `seal_cost`): logs 102 → 66 MB, a restore through
  400 logs 1.88 → 1.88 ms, the seal of 2,000 chunks 0.157 → 0.218 ms.

PR 2 against PR 2 + PR 3 on main `2cd5f151a` (engines `segpr`, `codec`), interleaved, **load 6–8**
(`raw/table-pr3.txt`, `raw/restore-pr3.txt`):

| doc | undo logs | malloc in use | footprint peak | edited page p50 / p95 | restore (median) |
|---|---|---|---|---|---|
| infdesc | 382 → **223 MB** | 532 → **373 MB** | 1.1 → **0.78 GB** | 53.8 / 96.1 → 55.1 / 92.5 ms | 36.1 → 36.5 M instr |
| full-100 | 49 → 30 MB | 138 → 120 MB | 261 → 238 MB | 15.4 / 20.1 → 15.7 / 21.6 ms | 13.1 → 12.8 M instr |
| long-deck | 43 → 26 MB | 126 → 109 MB | 248 → 224 MB | 38.7 / 42.2 → 38.7 / 41.9 ms | 19.8 → 19.8 M instr |

Edited-page instructions: +0.4–1.8 % at p50 (the seal's packing). The steady footprint of
infdesc read 242–257 MB in both runs: the system had compressed the idle host (see the caveat
above), so the table gives the peak and the heap.

**Soundness** (engine `codec` = `ead2c608a`; the next commit only renames a bit trick for clippy;
`raw/soundness/summary-pr3.txt`): every restore reads packed logs, so the fixtures ran too.

| sweep | documents | compiles | mismatches |
|---|---|---|---|
| A: 30 letters + reverts | plain-120, full-100 | 120 (114 converged) | **0** |
| C: 12 structural edits + reverts | refs-120, full-100 | 46 | **0** |
| D: 8 interleaved edits | refs-120, full-100 | 29 verified + 6 interrupted | **0** |
| A: 4 letters + reverts per fixture | every parity fixture (86) | 688 (100 converged; 1 log differs in accounting only) | **0** |

`scripts/gate.sh pr` at `47d3748e9` passed (`raw/gate-pr-3.txt`).

## Where it stands (all three PRs, against main)

| doc | target | main (Phase 1) | PRs 1–3 |
|---|---|---|---|
| infdesc | ≤ 0.5 GB steady | malloc 1,339 MB, peak 1.8 GB | malloc **373 MB**, peak **0.78 GB** |
| full-100 | ≤ 150 MB | malloc 186 MB, footprint 292 MB | malloc **120 MB**, footprint **238 MB** |

What is left on full-100 beyond the heap: the word space (27 MB dirty, 15 MB compressed), the slab
(5 MB), 61 MB of TeX Live tables (kpathsea's `ls-R` 37 MB, the font map 24 MB) and the page cache
(11.5 MB). The 150 MB target needs those next: a compact `ls-R` and font map, and a smaller page
cache (BELIEF: about −40 MB together; not built).

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
