# MEM-RESEARCH: what else cuts memory without costing speed (2026-10-06)

Lane **MEM-RESEARCH** (kabir-claude subagent, research only; the Commander assigns the work).
Owner, 2026-10-06: *"we need to optimize memory in general … all the ways we can optimize memory
usage without sacrificing performance."* DESIGN.md §5.2 (word space, undo logs, retention budget),
§5.6, §6. Earlier lanes: P4-MEMORY (`docs/evidence/p4-memory-2026-09-30/`), MEM-FOOTPRINT
(`docs/evidence/mem-footprint-2026-10-04/`), the memory PRs #1573, #1575, #1619 and #1620.

**Labels.** VERIFIED: measured here; the command and raw file are named. ESTIMATE: arithmetic on
measured sizes, shown. BELIEF: inferred, not measured. Every saving is resident memory (Linux RSS);
on macOS `phys_footprint` counts the same anonymous pages (see "macOS").

**Machine and build.** NixOS PC (Ryzen 7 7800X3D, 30 GB), shared with other lanes: load 3–79
during these runs. Memory figures do not depend on load; wall times do, so costs are given in
**instructions** (`edited_instr_k`, the PMU counter in DONE), not milliseconds. The engine is
`origin/main` `b76f057f1` (with #1620) merged with #1575 (which carries #1573) and #1619:
`8553c7d99`, on the PC in `~/code/flashtex-memr`. Builds: production (`memr`), `mem-stats`
(`memr-ms`, heap by tag), and a prototype (`proto`, `proto2`: dump hooks only, kept on the PC in
`~/code/flashtex-memr-proto`, not pushed).

## Ranked findings

Savings are at 1,000 pages (plain-1000 / full-1000) and on Infinite Descent (infdesc, 592 pages);
infdesc ×2 (1,152 pages here) in §3. "Cost" is on the keystroke path, T7 p50/p95.

| # | Change | Saving MB plain / full / infdesc | Keystroke cost | Size | Risk | Lane / files |
|---|---|---|---|---|---|---|
| 1 | **Retention: far from the cursor keep one page checkpoint in k** beyond ±16 pages, plus **idle re-densify** where the caret or viewport goes | k=2: **95 / 121 / 83**; k=4: **148 / 190 / 136** (VERIFIED offline: the dumped logs merged as `retain` merges them) | T7 rows 0 (BELIEF: the first key of a phase is not sampled and later keys restart close by); first key after a far jump without re-densify: up to k−1 extra pages (40–100 M instructions each on full-1000) | M | soundness of merges already gated (#1573/#1619); re-densify needs a new idle job | `incr.rs` (`thin`), host idle loop |
| 2 | **Page cache compressed when idle** (zstd-1 per page, pages away from the viewport) or spilled to a file | **50 / 50 / 13** (zstd); file: **~100 / ~98 / ~25** resident | 0 (after DONE; resends decompress 0.1 ms/page) | S | low; compare by content hash, not body | `host/resident.rs` (`Cached`) |
| 3 | **Compact undo-log headers** (chunk gaps as varints, masks as runs, an offset every 16 deltas) | **23 / 33 / 31** (VERIFIED offline) | seal +a few ns per delta (BELIEF ≤ 0.5 % of edited-page instructions) | S–M | low; round-trip tests | `arena.rs` |
| 4 | **jemalloc under `logalloc` on Linux** (instead of glibc for the non-log heap) | peak **27 / 47 / –** (VERIFIED, 3 rounds; steady ±10) | 0 (cold open 325.1 G vs 325.1 G instructions; edited page within noise) | S | low; a dependency (BSD licence) | `logalloc.rs`, `host/main.rs` |
| 5 | **Land #1493 and #1505** (kpathsea's duplicate instance; per-restore zlib leak; `reloc`/`taken` growth) | **9.6 per host** (Linux, VERIFIED heaptrack), 18.2 on the Mac (#1493's own measurement); growth of +26.5 MB per 100 edits (#1505's) bounded | 0 (fewer start-up lookups) | done (open PRs) | reviewed there | resolver, host |
| 6 | **Font map parsed on demand** (an index of `pdftex.map`, an entry parsed at its first lookup) | **19.5 per host** (VERIFIED heaptrack size; saving ESTIMATE ≈ 18) | 0 (first compile does less work) | M | pdfTeX's duplicate and `\pdfmapfile`/`\pdfmapline` rules: parity-gated | `pdftex/mapfile.rs` |
| 7 | **Dead buffer words not logged** (`pdf_op_buf`, `pdf_os_buf`, `buffer` beyond their live pointers) | **16 / 17 / 10** (VERIFIED offline, share of log words) | negative: fewer barrier copies | M | convergence must ignore those words (`incr::dead_word`) | `arena.rs`, `checkpoint.rs`, `incr.rs` |
| 8 | **macOS: undo-log blocks in their own mappings** (#1620's `logalloc` ported) | Mac only; BELIEF ≥ 10 % of the peak footprint (xzone's deferred reclaim) | 0 (Linux A/B equal instructions) | S–M | low | `logalloc.rs` |
| 9 | **The word space's untouched S₀ pages file-backed** (S₀ as a sparse, page-aligned image mapped `MAP_PRIVATE`) | **26 / 25 / –** chunk-level (VERIFIED: written only before the first checkpoint); page-level BELIEF 15–25 | ~0 (minor faults instead of a copy at reopen) | M–L | S₀ format change | `host/mod.rs`, `os.rs`, `arena.rs` |
| 10 | **App: glyph-index cache bounded and cleared** (`sourceIndexes`) | ESTIMATE 116 (infdesc) / 300 (dense 1,000 pages) | negative (fewer indexes built) | S–M | low; also fixes a stale-index bug | `EngineV3Session.swift`, `EngineV3SourceMap.swift` |
| 11 | App: memory-pressure handler; image keys instead of strong refs; font programs shared per hash; purgeable off-screen IOSurfaces | ESTIMATE 20–960 depending on content | 0 until pressure | S–M each | low–medium | `DL3Renderer.swift`, `EngineV3Preview.swift` |
| 12 | Slab: give back free chunks above a high-water mark at idle | **2.5 / 8 / 12** (x2: 22) | ~0 (only above the high-water mark) | S | low | `arena.rs` (`Slab`) |

Not for Balanced mode (they cost speed); **Low-Memory-mode candidates**: zstd on cold logs (×0.5 of
those logs, decompression on far restores), a smaller default budget (§5.2's owner call), XOR or
arithmetic coding of log words against the next state (−12 % of words but a full sequential
rewind), app decoded pages evicted to wire bytes, no keep-warm. Rejected: content-hash dedup of
log chunks (−1–2 %), huge pages for the word space (more RSS), dropping the spare output-tail
buffers (they exist because zero-filled buffers cost 9–12 ms per restore, P4-MEMORY §2).

**Open question, not measured:** the display list's side table `dl_side` is 29–31 % of all logged
words (46–64 MB at 1,000 pages and on infdesc). A narrower table (a 32-bit span per `mem` word
instead of a 64-bit word) or one that records spans only for nodes that reach a page could cut that;
it needs its own design against the span gate (P4-MEMORY §2).

## 1. Where the memory is now (VERIFIED)

T7 (six in-body phases, typing@50ms, preamble; production build, `scripts/t7.sh`), host peak RSS:

| doc | this stack | Commander's 10-06 figure | main before the memory PRs |
|---|---|---|---|
| plain-1000 | **595 MB** | ~580 MB | 1.2 GB |
| full-1000 | **677 MB** | ~650 MB | 1.4 GB |

The parts after typing at the middle and the end, then 8 s idle (`mem-stats` build, `scripts/memr.py`;
`raw/memstats-*.txt`):

| part | plain-1000 | full-1000 | infdesc |
|---|---|---|---|
| RSS steady / peak | 482 / 542 | 573 / 651 | 468 / 549 |
| undo logs (`sealed_bytes`) | 220 (46 %) | 281 (49 %) | 233 (50 %) |
| page cache (DL3 bodies, tag `dl`) | 106 (22 %) | 104 (18 %) | 25 |
| C heap held but free (glibc `malloc_held − malloc_in_use`) | 47 | 70 | – |
| word space resident | 31 | 38 | 44 |
| engine (font map 19.5 + kpathsea 9.6 + rest) | 25 | 31 | – |
| spare output tails (tag `branch`) | 13 | 14 | 10 |
| slab resident (live) | 4.6 (2.1) | 10.4 (2.9) | 21.6 (9.2) |
| prepared restore | 2–4 | 3–5 | 18 |
| pdfTeX C state copies (`cow`) | 1 | 7 | – |
| checkpoint records | 6 | 3.4 | – |
| second kpathsea instance (#1493) | 9.6 | 9.6 | 9.6 |

(infdesc from the production build's `mem`, `raw/inf-base.txt`, which has no heap tags. On this
stack every infdesc keystroke reported status `error` and re-typeset 526 pages without converging,
90–130 s each at load 15–36; not investigated here, outside the lane. Its logs are therefore one
run's, without a pending branch.)

**The logs, word by word** (`scripts/logana.py` on a dump of every sealed log at the steady state;
`raw/logana.txt`):

| | plain-1000 | full-1000 | infdesc |
|---|---|---|---|
| logs / deltas / words | 1,011 / 1.77 M / 39.7 M | 1,013 / 2.31 M / 51.6 M | 730 / 2.15 M / 44.5 M |
| bytes: delta headers + packed words | 40.4 + 178.7 | 52.8 + 227.6 | 49.1 + 181.9 |
| words by array: `mem` / `dl_side` / `pdf_op_buf` | 53 / 30 / 15 % | 53 / 31 / 12 % | 58 / 29 / 8 % |
| bytes by tenth of the chain (MB) | 22 each | 26–29 each | 7–29 |

The logs are spread evenly along the document: every page checkpoint far from the cursor costs
as much as one near it.

## 2. Details

### 2.1 Retention far from the cursor (#1)

#1573 keeps one checkpoint per page beyond `DENSE` = 16 pages from the cursor. Merging k adjacent
logs keeps, for each word, the oldest value (`retain`), so the merged log holds the union of the
words, not the sum. Simulated on the dumps (`logana.py`, "keep 1 checkpoint in k"; whole chain):

| k | plain-1000 | full-1000 | infdesc |
|---|---|---|---|
| 1 (now) | 219 MB | 280 MB | 231 MB |
| 2 | 122 | 155 | 143 |
| 4 | 66 | 85 | 87 |
| 8 | 35 | 45 | 51 |

With the ±16-page window kept dense (3 % of a 1,000-page chain) the saving is the table's times
0.97 (ESTIMATE): k=2 saves 95 / 121 / 83 MB, k=4 148 / 190 / 136 MB.

**Cost.** A keystroke restarts from the newest checkpoint before the edit. In a thinned region the
first keystroke restarts up to k−1 pages earlier: one page of full-1000 is 40–100 M instructions
(edited page p50 91.5 M, `raw/t7-base.txt`; #1573 measured +39 M on full-100 for one restart
moved to a page start). Later keystrokes at the same place restart close by, because the rerun
takes a checkpoint at every page it typesets. T7 drops each phase's first keystroke, so its rows
do not move (BELIEF).

**Re-densify (new).** The host knows the viewport (DL3 protocol) and can be told the caret line.
When either moves into a thinned region and the host is idle, it restores the kept checkpoint
before it and re-runs the next k pages, taking their checkpoints again. With no edit in between
the run is deterministic, so the new checkpoints are the old states (sound; the convergence test
would confirm it page by page). The cost is k−1 pages of idle CPU per jump (≈ 10–30 ms), before
the user types. This is the "smarter density policy for the same latency": dense where the user
is, sparse elsewhere, with the gap closed while the user reads.

### 2.2 Page cache (#2)

The host keeps every page's DL3 body (`Cached.e.body`) to resend pages a client lacks. 98–100 MB at
1,000 pages (100 KB per page), 25 MB on infdesc (`raw/pageana.txt`, `raw/memstats-*.txt`).

| | full-1000 | plain-1000 | infdesc |
|---|---|---|---|
| bodies | 97.6 MB | 99.7 MB | 24.7 MB |
| zstd-1 per page | 0.49 | 0.50 | 0.49 |
| zstd-3 per page, trained 110 KB dictionary | 0.43 | 0.44 | 0.42 |
| compress / decompress (zstd -b1) | 420 / 1,080 MB/s | 416 / 1,087 MB/s | 420 / 1,046 MB/s |

Compressing a 100 KB page takes 0.24 ms, so it must not run before the edited page is out: compress
pages after DONE, starting with those farthest from the viewport. The equality check before a
version bump (`c.e.body == e.body` when the hashes match) becomes the hash alone (SHA-256 of the
body). A reconnecting client gets pages decompressed at 0.1 ms each. Spilling the bodies to a file
in the work directory (page cache, not anonymous memory) removes them from RSS and the macOS
footprint altogether, at the cost of a write per changed page.

### 2.3 Compact log headers (#3)

Each delta carries a 24-byte header (`Delta { c: u32, at: u32, mask: [u64; 2] }`): 19–21 % of the
log bytes. The deltas of a log are sorted by chunk, so the chunk is a small gap (1–2 bytes as a
varint), and a mask has few runs. Coded as gap + runs (at most 16 bytes) with a byte offset every
16 deltas for the restore's skips: 40.4 → 17.7 MB (plain), 52.8 → 20.2 (full), 49.1 → 18.6
(infdesc). The restore already walks a log's deltas in order (`apply_under`); the decode adds a few
instructions per delta. #1575's packing measured +0.4–1.8 % of edited-page instructions for a
heavier change; this one should be smaller (BELIEF).

### 2.4 Allocator (#4)

At the steady state glibc holds 47–70 MB more than is in use (`malloc_held − malloc_in_use`,
table above), and its 64 MB arenas are the largest group in smaps at the full-1000 T7 peak (315 MB
of 677). The same production host under `LD_PRELOAD` (`scripts/alloc2.sh`: open, 6 letters at the
middle, 3 sentences, 4 letters at the end; `raw/alloc.txt`; two hosts at a time, load 17–26):

| | full-1000 peak / steady | plain-1000 peak / steady | cold open instructions | edited page p50 (letters at the middle) |
|---|---|---|---|---|
| glibc (now); full 3 rounds, plain 2 | 650, 649, 649 / 568, 565, 577 | 538, 536 / 469, 481 | 325.1 G | 96.7–98.5 M |
| **jemalloc 5.3.1**; full 3, plain 2 | **601, 605, 602** / 563, 561, 567 | **512, 507** / 477, 473 | 325.1 G | 96.4–109.2 M |
| mimalloc 3.3.2, 1 round | 641 / **601** | – | 323.0 G | 95.5 M |
| glibc, `MALLOC_ARENA_MAX=2`, 1 round | 648 / 563 | – | 324.9 G | 97.7 M |

jemalloc lowers the peak by 47 MB (full) and 27 MB (plain) with the steady state unchanged; the
instructions of the cold open are equal to 0.1 %, and the edited-page medians move both ways within
the restart-distance noise. mimalloc is worse at rest; fewer glibc arenas change nothing. The peak
is the transient buffers of the convergence test and the jump (#1620 counted ~130 MB of ≥ 64 KB
blocks per keystroke on full-1000), which jemalloc's size classes and decay reuse without growing
(BELIEF: the mechanism is not measured, the peaks are).
Ship it as the `System` that `logalloc::HostAlloc` falls back to (Linux), or as the global allocator
with `logalloc` on top; the undo logs keep their own mappings either way. macOS keeps libmalloc
(different behaviour, §2.7).

### 2.5 Fixed costs per host (#5, #6)

heaptrack (`raw/heaptrack-full-100-peak.txt`, `raw/heaptrack-full-1000-peak.txt`; heap at its peak by
owner, `scripts/fgagg.py`):

| owner | full-100 | full-1000 |
|---|---|---|
| page cache (`dl_shipout_end` → `encode`) | 10.3 | 102.9 |
| undo logs below 64 KB (`retain`, `seal`) | 5.1 | 51.8 |
| font map (`fm_read_info`, `avl_do_entry`, `read_field`) | 19.5 | 19.5 |
| kpathsea `ls-R`, the engine's instance | 9.6 | 9.6 |
| kpathsea `ls-R`, `server::prepare`'s instance (never freed) | 9.6 | 9.6 |
| convergence test (`diff_branch`) | 13.5 | 9.5 |
| output tails (`read_tail_into`, `move_tail`) | – | 22 |

Every open document has its own host, so these multiply by the open documents. #1493 already
removes the second kpathsea instance (it starts kpathsea at the first lookup). The font map is
46,659 lines (5.6 MB, TeX Live 2026) parsed into 19.5 MB of entries and trees; a document uses tens
of fonts. An index from TFM name to file offset (≈ 1 MB) and parsing at the first lookup keeps
pdfTeX's semantics if the lookup applies the same first-wins and `+`/`=`/`-` rules.

### 2.6 Dead buffer words (#7)

`pdf_op_buf` is pdfTeX's output buffer; between `pdf_ptr` and its end the bytes are dead. A page
rewrites the whole buffer, so every page checkpoint logs it again: 12–15 % of all logged words,
14.4–14.5 MB at 1,000 pages. With `pdf_os_buf` and TeX's `buffer` beyond their pointers: 16 / 17 / 10
MB. Not logging them needs the checkpoint's pointer (kept in its record) at seal time and the
convergence test and state hash to skip those words, as `incr::dead_word` already does for the side
table's boundaries. It also saves the write barrier's chunk copies there.

### 2.7 macOS (#8, OS view)

From Apple's sources (xnu `task.c`, `vm_map.c`, `vm_object.c`, `vm_reclaim.c`; libmalloc), read by a
sub-agent, not measured:

- `phys_footprint` counts anonymous dirty pages, **compressed pages at their full size** (so
  compression never lowers it), IOKit mappings, non-volatile purgeable memory and page tables. Clean
  file-backed pages do not count; a `MAP_PRIVATE` file page counts once written.
- `MADV_DONTNEED` does nothing to the footprint on macOS; `MADV_FREE_REUSABLE` drops it at once
  (`MADV_FREE_REUSE` before reuse); `munmap` drops it; volatile purgeable memory does not count.
- libmalloc's xzone allocator keeps freed spans counted until the kernel trims them, which it does
  only once they reach 10 % of the process's lifetime peak (5 % at warning, 1 % at critical). That
  explains `malloc_zone_pressure_relief` returning 0 (MEM-FOOTPRINT) and a steady footprint that
  moves with pressure. The MEM-FOOTPRINT note that the steady figure moved "because idle pages get
  compressed" does not hold: compression does not change the footprint.

So: port `logalloc` to macOS (`mmap`/`munmap` per log block of 64 KB and more, `realloc` by copy):
freed logs leave the footprint at once, as #1620 measured on Linux (T7 peak 783–801 → 648–649 MB
on full-1000, 654–658 → 558–582 MB on plain-1000, instructions equal). A purgeable page cache (#2) is a second option on macOS. The test recipe
for a team Mac is `scripts/memfp.c` (each technique on 512 MB: footprint after, refault cost per MB):

```
clang -O2 -Wall -o memfp memfp.c
for c in munmap remap dontneed free reusable reusable_noreuse purgeable file_shared file_private malloc; do
  nice ./memfp $c 512; done 2>&1 | tee memfp-$(sw_vers -productVersion)-$(sysctl -n hw.model).txt
MallocReportConfig=1 ./memfp malloc 512
HOLD=60 ./memfp purgeable 512 &   # meanwhile: memory_pressure -l warn (never -l critical)
WAIT=90 ./memfp compressed 512 &  # then memory_pressure -l warn; Ctrl-C after ~60 s
```

### 2.8 Word space (#9)

532 MB reserved, 30–38 MB resident (`raw/wordspace-*.txt`, `proto2`'s count of chunks written after
the first checkpoint):

| | plain-1000 | full-1000 |
|---|---|---|
| touched | 29.5 MB | 36.8 MB |
| written after the first checkpoint | 3.8 MB | 11.8 MB |
| written only before it (format, preamble) | **25.7 MB** | **25.0 MB** |

The format's tables (`font_info`, `eqtb`, `hash`, `str_pool`, `trie`, ~5 MB each) are read, not
written, after S₀. Mapped from a page-aligned S₀ image with `MAP_PRIVATE`, those pages would be clean
file pages: out of the macOS footprint, droppable on Linux. Today S₀ stores only present 1 KB chunks
at packed offsets (`host/mod.rs` `read_s0`), so the format would change. Page granularity (16 KB on
Apple Silicon) loses some of the 25 MB. Sparse/lazy commit is already there (anonymous `mmap`);
`MADV_DONTNEED` on TeX's free `mem` would save little (mem resident 3.5–6.3 MB). Huge pages would
raise RSS: not for Balanced.

### 2.9 App side (#10, #11)

Code reading only (sub-agent; nothing runs on the owner's Mac). Measured app numbers in the repo:
198 MB peak on infdesc (`docs/evidence/infdesc-app-2026-10-03/`), tiles 41 MB of a 108 MB
footprint on dense.tex (`docs/evidence/v3-zoom-tiles-2026-09-30/`). Paths under `apps/mac/Sources/`.

1. **Glyph index cache unbounded** (VERIFIED in code): `sourceIndexes` (`FlashTeXMac/EngineV3Session.swift:226`)
   is filled for every page that `place(path:line:col:)` scans (`FlashTeXMac/EngineV3SourceMap.swift:55-59`);
   caret-follow calls it on every recompile. Never cleared on stop, project switch or a shrinking
   page count, so it can answer with another project's glyphs. ESTIMATE ~100 B per glyph: 116 MB on
   infdesc, 300 MB on a dense 1,000 pages. Fix: a span → first-page map built as pages arrive, a full
   index only for the matching page, an LRU of ~16, cleared with `pages`.
2. **No memory-pressure handling anywhere**: one `DispatchSource.makeMemoryPressureSource` that drops
   caches on warning costs nothing until the system asks.
3. **Decoded images held by pages past the cache bound** (`FlashTeXPreviewV3/DL3Renderer.swift:308`,
   `:356`): pages should hold keys and resolve through the bounded cache.
4. **Font programs copied per key** (`DL3Renderer.swift:82-83`): one `CGFont` per program hash.
5. **Off-screen page IOSurfaces** (`FlashTeXMac/EngineV3Preview.swift:852-867`): mark volatile
   (purgeable) outside the visible rect; never display a volatile surface.
6. Decoded pages kept forever (`EngineV3Session.swift:184,186`; ESTIMATE 55 / 140 MB): a tighter
   `DL3Item` layout saves ~20 % at no cost; evicting far pages to wire bytes is Low-Memory mode.
7. PDFs read with `Data(contentsOf:)` (`DL3Renderer.swift:821-823`); the output PDF could be mapped.

Team-Mac checks: `footprint -p`, `vmmap --summary`, `heap <pid> | grep -E 'DL3GlyphRef|DL3SourceIndex'`
before and after a caret-follow scan, Instruments VM Tracker for IOSurface.

## 3. Infinite Descent ×2

`infdesc-x2.tex` (#1646; 1,152 pages here), production build, `scripts/x2.sh`, `raw/x2-base.txt`.
VERIFIED for the open only: the open took 2,443 s of wall time at load 45–79 (not a speed figure),
and the three edit steps then failed at once in `dl3-keys` (exit 1 within 2 s, so no keystroke
reached the host; the book's chapters are `\input` twice, which the edit placement may not handle;
not investigated).

| after the open | MB |
|---|---|
| RSS peak / after it | **979** / 937; 920 after 8 s idle |
| undo logs (1,807 checkpoints) | **599** (64 % of RSS) |
| page cache | 48 |
| word space resident | 56 |
| slab resident (live) | 22.5 (0.6) |
| spare output tails | 19 |
| prepared restore | 32 (after the first request) |

The logs are a larger share here because the open is one long run whose segment checkpoints are
thinned only around the cursor, which is at the start. Applying the ratios measured on infdesc
(ESTIMATE): compact headers −80 MB (#3); k=2 retention −250 MB, k=4 −380 MB (#1); page cache
−24 MB (#2); slab −22 MB (#12); dead buffers −25 MB (#7). Together with #1 at k=2 that is about
−400 MB, i.e. x2 at roughly 0.55 GB instead of 0.98 GB at the open's peak.

## Reproducing

```
# PC: build (production; mem-stats with FEATURES=mem-stats; the prototype from ~/code/flashtex-memr-proto)
scripts/build.sh memr; scripts/build.sh memr-ms mem-stats
scripts/t7.sh memr base                       # T7 with an smaps sampler -> ~/ib-memr/t7-base
python3 scripts/t7instr.py ~/ib-memr/t7-base  # instructions, RSS, logs per phase
scripts/inf.sh memr inf-base infdesc.tex      # infdesc steps; scripts/x2.sh for infdesc-x2.tex
python3 scripts/memr.py ENGINE SRC MAIN OUT --step FILE:AT:KIND:KEYS ...   # any document
scripts/dump.sh TAG SRC MAIN STEP...          # prototype build (dump hooks: ~/memr/patch_dump.py, patch_pages.py on the PC)
python3 scripts/logana.py DUMP.logs; python3 scripts/pageana.py DUMP.pages
scripts/alloc2.sh ENGINE DOC; python3 scripts/allocsum.py OUT...   # allocator A/B (LD_PRELOAD wrappers)
heaptrack (nixpkgs) around the host: ENGINE/flashtex-host as a wrapper; scripts/fgagg.py on
  heaptrack_print --flamegraph-cost-type peak -F OUT
```

Run each under the PC's `~/.local/bin/ftx-run TIMEOUT CMD` (the agents' slice), at most two hosts at
a time. The heaptrack wrapper outlives the harness's stop: the real host keeps running and has to be
stopped by its PID.
