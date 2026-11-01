# Where the first page's time and the host's memory go (2026-10-04)

The low-load latency run (`../README.md`, PR #1511) left the cost between the end of the compile
and the first page on the socket partly unexplained: p50 33–44 ms at 300 pages and 112–298 ms at
999. These are the Commander's follow-ups: stage counters, the largest stage, and what dominates
the heap at 999 pages. Machine: mac-m1max-a (M1 Max), **non-reference**. Each row records its
load average. The runs waited for load < 20 and aborted when it stayed above 25 for 15 s
(`run7.sh`; `load.jsonl` has the 5-second samples). Documents: Track A's generator,
`typing_bench gen DIR 415` (300 pages) and `1385` (999 pages). The `-noimg` copies replace each
`image("fig.svg", width: 45%)` with `rect(width: 45%, height: 2cm)`, giving 291 and 969 pages.

## Stage counters

`DONE.stages` (spec §11.9) splits the time into:
- the previous compile's work after its `DONE`: `prev_lock_ms` and `prev_evict_ms`;
- how long the `COMPILE` waited for the host (`queued_ms`);
- the edits, the compile and the page hashes;
- for the first page: its export, reading it, conversion, span resolution and socket write.

The bench (`typing_bench`) reports each stage's p50/p95, plus `client_extra_ms`: the client's
first-page time minus the host's.

Stage p50s (ms), seeded, EDITMID:

| Stage | 300 pages | 999 pages |
|---|---|---|
| waiting behind the previous compile's `comemo::evict` (`queued_ms` = `prev_evict_ms`) | 20 | 66–71 |
| page hashes (8 threads) | 6–7 | 18–22 |
| first page's source spans, before the fix below | 4.0 (EDITMID), 8.7 (EDITEND) | 14 (EDITMID), 17 (EDITEND) |
| first page's positions (one-page export 1.1–1.2 + reading it 0.9) | 2.1 | 2.2 |
| first page's conversion, edits, socket write | 0.5 + 0.6 + 0.03 | 0.5 + 1.8 + 0.03 |

Together these account for the gap: 31 of 33 ms at 300 pages and about 105 of 105 ms at 999
(`stages.jsonl`). **The largest stage is the previous keystroke's eviction**, which runs on the
server thread after `DONE` while the next `COMPILE` waits. The bench types back to back. When
typing, a key that arrives during an eviction waits for it the same way.

## Fixed: span resolution (−4 to −17 ms)

Typst's `Source::range` finds a span by walking the root's children one by one. A 3 MB file has
tens of thousands of them, so the walk is slow for every span near the end of the file. The
host now builds, once per compile and file, an index of the root's children (span number and
start offset) and binary-searches it, and each level below.

The unit test `span_index_agrees_with_typst_for_every_node` compares it with Typst's own lookup
for every node of a 20,000+-node source, and checks that numbers no node has return nothing.

Measured, p50 (ms):

| Pages | Before | After | File |
|---|---|---|---|
| 300 | 0.45 / 4.0 / 8.7 (EDITSTART / MID / END) | 0.16 / 0.16 / 0.16 | `spanix.jsonl` |
| 969 | 14–17 near the end | 0.35–0.41 | `noimg2.jsonl` |

## Not fixed: eviction (two attempts)

1. **Eviction on its own thread: rejected.** The next compile no longer waits (`queued_ms` ≈
   0.04), but it runs while comemo's caches are being evicted and gets much slower (`ab.jsonl`):

   | Pages | Typst compile p50, sync eviction (ms) | Typst compile p50, async eviction (ms) | First page p50, sync → async (ms) |
   |---|---|---|---|
   | 300 | 120–139 | 215–230 | 158–219 → 232–249 |
   | 999 | 646–747 | 1,780–1,783 | 756–883 → 1,810–1,832 |

   The code was removed.
2. **What makes eviction (and memory) large: the DONE.pdf compile.** While a page the client
   holds is INCOMPLETE, every compile also runs the standard compile and typst-pdf's whole
   export for `DONE.pdf` (spec §11.9; DESIGN §15.3). That doubles comemo's caches. In this
   corpus every page with an SVG figure is INCOMPLETE: images and islands (E5/E6) are not
   produced yet.

   The `-noimg` copies have the same layout with rectangles, so no page is INCOMPLETE:

   | | 300 pages: with images → without | 999 → 969 pages: with images → without |
   |---|---|---|
   | eviction p50 (ms) | 20 → 7 | 66–75 → 23–28 |
   | RSS after the edits (GB) | 1.50–1.53 → **0.83** | 4.49–4.73 → **2.72** |
   | `DONE` after the first page (ms) | +420–530 → +0 | +1,650–2,060 → +0–3 |
   | first page p50 (ms) | 137–219 → 125–146 | 756–883 → 693–755 |

   Sources: `noimg.jsonl`, `noimg2.jsonl`, `spanix.jsonl`, `ab.jsonl`.

   So the fix for the largest stage, and for the memory miss below, is to stop the per-keystroke
   `DONE.pdf` compile. Either the pages stop being INCOMPLETE (E5 islands for SVG images, E6 for
   raster images: next in the lane), or `DONE.pdf` is deferred to the idle check, which already
   runs a standard compile. The second option changes `DONE`'s meaning (§11.9) and DESIGN
   §15.3's "DONE.pdf from the standard compile", so it is **for the Commander to decide**; it is
   not done here.

What is left after the compile without INCOMPLETE pages, p50 (ms):

| Pages | Gap after the compile | Eviction | Hashes | Positions | Spans | Rest |
|---|---|---|---|---|---|---|
| 291 | 18–19 | 7 | 8 | 2 | 0.15 | ~1 |
| 969 | 50–75 | 23–28 | 23–25 | 2 | 0.4 | ~2 |

**Hashes cannot be skipped by identity.** No page's frame is the previous document's own:
0 of 19,980 pages were pointer-equal across 20 seeded compiles (`mem-p1000.json`,
`same_frames`), because Typst builds each page's frame anew.

## Memory at 999 pages: what dominates

`examples/mem_probe.rs` runs the host's compiles in one process:
- a cold compile, then every page converted;
- 20 seeded keystrokes at EDITMID, each followed by `comemo::evict(3)`;
- with `--done-pdf on`, also the standard compile and the whole export after each keystroke, as
  the host does while a page is INCOMPLETE.

It then measures the heap in use (macOS `malloc_zone_statistics`) as each part is released.

| 999 pages (MB) | Host as now (`--done-pdf on`) | Without INCOMPLETE pages (`off`) |
|---|---|---|
| RSS after the keystrokes | **4,470** | **2,580** |
| heap allocated from the system | 4,494 | 2,614 |
| heap in use | 3,424 | 1,315 |
| freed by releasing the retained documents | 0 (shared with the caches) | 0 |
| freed by releasing the connection's tables | 14 | 13 |
| **freed by clearing comemo's caches (`evict(0)`)** | **3,036** | **1,004** |
| freed by releasing the World (sources, files) | 28 | 27 |
| freed by releasing the fonts | 2 | 2 |
| still in use after all of that | 345 | 268 |

- **comemo's caches dominate** the heap in use, at 89 % and 76 %. The DONE.pdf compile triples
  them.
- Retained frames are no extra cost, since they are the caches' own outputs. Page hashes are
  16 bytes a page; the connection's tables are 13–14 MB and fonts 2 MB.
- **The allocator holds 1.1–1.3 GB of free memory** on top of the heap in use (RSS minus heap in
  use). `malloc_zone_pressure_relief` returns none of it (0 MB, `mem-p1000-relief.json`), so it
  is fragmentation.

At 300 pages (`mem-p300-pdf-*.json`) the probe gives:

| 300 pages | RSS (GB) | comemo caches (MB) | eviction p50 (ms) |
|---|---|---|---|
| with the DONE.pdf compile | 1.40 | 924 | 31 |
| without | **0.80** | 302 | 13 |

**Conclusion (measured):** with no INCOMPLETE page, the host is inside the memory gates:
- 0.80–0.83 GB at 300 pages (gate 1.1 GB);
- 2.6–2.7 GB at 969–999 pages (budget 3–4 GB).

The misses recorded in the T1 gate table come from the DONE.pdf compile that INCOMPLETE pages
trigger.

**Belief, not measured:** a different allocator (e.g. mimalloc) would recover part of the 1.1–1.3
GB of fragmentation. That would add a dependency, so it is not tried here.
