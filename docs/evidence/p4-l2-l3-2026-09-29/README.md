# P4-L2-L3: per-page checkpoints, restart and converge, viewport first (2026-09-29)

Lane P4-L2-L3 (DESIGN.md §5.2 L2, §5.3 L3, §5.4 L4, §1.2 targets, §12 P4). Branch
`agent/kabir-claude/p4-l2-l3` (from `agent/kabir-claude/p4-l1-foundation`, with #1202 and
`origin/main` merged in), draft PR #1223. Raw output in [`raw/`](raw/), the drivers in
[`scripts/`](scripts/) (they expect the engines in `/tmp/p4l2/<name>/` made by
`scripts/mkeng.sh`, the documents made by `scripts/gen.py`; see *Reproducing*).

**Host:** mac-m5pro-kabir, Apple M5 Pro, 24 GiB, macOS 26.6.2, rustc 1.98.0, release
builds. The machine was shared with other agents and a self-hosted CI runner for the whole
day: the 1-minute load average ran from 6 to 82 (`uptime` lines in `raw/`). **Wall-clock
latencies below are therefore upper bounds; the thread-CPU columns are the robust
measure**, and checkpoint costs come from `/usr/bin/time -l` cycles and instructions.

**Documents.** `scripts/gen.py` writes deterministic benchmark documents of 10, 100, 300
and 1,000 pages: *plain-N* (article, geometry, amsmath: prose, inline and display math,
sections) and *full-N* (the same plus hyperref, siunitx, cleveref, xcolor, footnotes,
labels and cross-references). The three documents of DESIGN Appendix B are used too:
*Hello* (`min`), *120 pages* (`long-full`, `\input{body.inc}`) and *953 pages*
(`long8`, `body.inc` input eight times).

## Results

### Against the lane's targets

| Target (DESIGN §1.2, lane) | Measured (verified unless marked) | Status |
|---|---|---|
| Edited page ≤ 16 ms p95, edit inside a paragraph, up to 1,000 pages | p95 wall / CPU over all 66 edits per document: plain-10 7.4 / 7.1, plain-100 6.0 / 5.4, plain-300 4.1 / 2.8, plain-1000 12.7 / 9.5; full-10 32.9 / 32.3, full-100 14.7 / 13.8, full-300 1,450 / 1,299 (5 `.aux` re-runs), full-1000 32.2 / 16.3 ms | met for *plain* at every size and full-100; not for full-10 (page-1 edits), full-300's `.aux` re-runs and full-1000 (CPU p95 16.3 ms, wall 32 ms on a loaded machine) |
| Reflowing edit: edited page ≤ 16 ms, later pages in the background, stale marked | the edited page is out at the same latency as a local edit (table below); the rest runs on (`compile N` returns at page N, `finish` continues, later pages reported stale). The compile *after* a reflow that moved a label reads a changed `.aux` and re-runs from the `.aux` point to the edited page: 0.27–3.9 s on pages 46–277 | edited page met; the `.aux` re-run is L5's (§5.5) |
| Reopen ≤ 100 ms to the first page | pre-warmed host: Hello 7–13 ms, full-1000 52–70 ms median (load 20–42); fresh process 150–450 ms | met in a pre-warmed host (the choice), not from a fresh process |
| Checkpoint ≤ 0.5 ms/page, memory within 1 GB on the 953-page document | **0.41 ms CPU/page, 1.75 M cycles, 2.76 M instructions** (quiet machine); 0.54–0.89 ms at load 12–41 with the same 2.78–2.92 M instructions; **236 MB** for all 954 page checkpoints | met (instructions are load-independent) |
| Incremental = from scratch, 50 random single-character edits per document | **8,500 compiles, 0 mismatches** (82 fixtures + Hello, 120 and 953 pages; PDF, log, aux, out, toc and terminal byte for byte); plus 1,700 reflowing sentence insertions, 0 mismatches | met |
| P-T1 82/82, lockstep 260/260, trip/etrip/drift, `scripts/gate.sh pr` | P-T1 **82/82**, P-T2 **82/82** (also in preview mode), lockstep **260/260**, trip, etrip, drift pass, `gate.sh pr` passes | met |

### Edited-page latency (`scripts/matrix.py`, `raw/latency-matrix.jsonl.gz`)

Final engine (64a710f6c), 18:38–18:49 local, load average 4 → 21 (`raw/matrix4-uptime.txt`;
the start of the run was the quietest hour of the day). Per document, region (the first,
middle and last tenth of the prose) and edit type: a typing session of 8 single-character
edits within 400 bytes of each other (each followed by its revert, 16 compiles), and 3
sentence insertions of twelve words with their reverts (6 compiles). *Edited page* = from
the `compile` command to the shipout of the first page whose bytes changed (when `compile N`
would return); *CPU* = the host thread's CPU time over the same interval.

| doc | start p50 / p95 ms (CPU) | middle p50 / p95 ms (CPU) | end p50 / p95 ms (CPU) | all 66: p50 / p95 wall | p95 CPU |
|---|---|---|---|---|---|
| plain-10 | 6.9 / 7.5 (6.6 / 7.1) | 2.2 / 3.4 (1.9 / 3.1) | 2.2 / 4.0 (1.9 / 3.8) | 2.3 / 7.4 | 7.1 |
| plain-100 | 5.6 / 16.2 (5.0 / 15.6) | 2.7 / 2.8 (2.1 / 2.2) | 3.7 / 4.0 (3.2 / 3.5) | 3.6 / 6.0 | 5.4 |
| plain-300 | 3.7 / 4.7 (2.5 / 3.6) | 3.3 / 3.5 (2.3 / 2.5) | 2.9 / 3.1 (2.2 / 2.3) | 3.3 / 4.1 | 2.8 |
| plain-1000 | 12.1 / 12.9 (9.1 / 9.7) | 5.3 / 5.8 (2.8 / 3.1) | 4.2 / 6.6 (2.4 / 4.8) | 5.4 / 12.7 | 9.5 |
| full-10 | 14.6 / 33.0 (14.1 / 32.4) | 8.5 / 15.8 (8.0 / 15.2) | 7.1 / 7.7 (6.7 / 7.1) | 8.5 / 32.9 | 32.3 |
| full-100 | 8.0 / 14.3 (7.2 / 13.5) | 7.5 / 268.7 (6.8 / 267.4) | 6.4 / 7.2 (5.7 / 6.6) | 7.5 / 14.7 | 13.8 |
| full-300 | 8.6 / 9.4 (7.1 / 7.8) | 7.2 / 1,450 (6.0 / 1,299) | 14.4 / 3,923 (8.6 / 2,656) | 8.9 / 1,450 | 1,299 |
| full-1000 | 14.5 / 32.2 (10.4 / 13.8) | 10.9 / 12.6 (8.0 / 9.0) | 21.0 / 42.6 (14.7 / 17.2) | 13.0 / 32.2 | 16.3 |

Reflowing (more than the edited page changed) against local edits, same run
(`raw/matrix4-summary.md` has every cell): the edited page costs the same; e.g. full-1000
reflowing p50 / p95 11.6 / 32.2 ms, local 16.8 / 29.4 ms.

What the numbers are made of (from the per-compile reports):

- **Re-running one or two pages.** TeX reads a whole paragraph before it breaks it, so a
  page is only a restart point if the paragraph holding the edit had not been read when it
  was taken: the restart is usually the page before the edited one, and the edited page
  costs the rest of that page plus itself. A page of the *full* documents costs 5–7 ms of
  CPU (hyperref, siunitx and footnotes; DESIGN §5.4 measured 5.9 ms on pdfTeX), a *plain*
  page ~1–2 ms. full-1000's last pages re-run two pages (14.7 ms CPU p50).
- **Page 1** re-runs from S₀ (or the `.aux` point), including LaTeX's begin-document work
  and the first use of each font: 7 ms for plain, 14 ms for full, 32 ms when the page-1
  edit also changed the `.aux`.
- **Restore** 0.6–3 ms p50 in most cells; 7.7 and 9.5 ms in full-1000's start and end
  sessions, the last of the run, as the load rose (belief: the restore's 8 workers waiting
  for cores; one of them took 95 ms);
  **finding the edit and the restart point** 0.5–2.5 ms, of which 1.8 ms on 1,000 pages is
  re-reading and comparing the 4 MB source (an editor that sends its edit would skip it).
- **Ten compiles took over 50 ms.** Nine are the compile after a reflow that moved a
  label (full-100 middle 3, full-300 middle 3 and end 2, plain-1000 middle 1): the `.aux`
  changed, so the run restarts at the `.aux` point and re-typesets every page up to the
  edited one (pages 46–277: 0.27–3.9 s). The tenth is the 95 ms restore above.

An earlier run of the same matrix on this engine at load 38–154 gave CPU p95s up to twice
these (every page was slower, E-cores and contention); it is not reported as a result.

### Checkpoint cost and memory (L2)

`scripts/ckcpu.py`: cold host runs of the 953-page document without and with a checkpoint
after every shipout, interleaved, three pairs, `/usr/bin/time -l`:

| per checkpoint, median of 3 pairs | 3677d9932 (quiet) | 64a710f6c, load 12–41 (`raw/ckcpu-final.txt`) |
|---|---|---|
| CPU (user + sys, with minus without, / 953) | **0.41 ms** | 0.62 ms (0.54–0.89) |
| cycles | 1.75 M | 2.16 M (2.06–3.23) |
| instructions | 2.76 M | 2.81 M (2.78–2.92) |
| sealing the log (thread CPU) | 0.23 ms | 0.30 ms |
| peak RSS without / with | 101 / 337 MB | 101 / 337 MB |

The checkpoint path did not change between the two engines (the later commits touch the
restart logic); the instruction counts agree, the cycles follow the load. The first
column's run was transcribed from its console output (`raw/ckcpu-3677d9932.txt`).
History: 0.60 ms when sealing was added (the seal compares the ~1,900 dirty 1 KB chunks of
a page with their pre-images, and by then both are out of the cache), 0.41 ms with a
prefetch two entries ahead. A host session on this document holds the logs of all ~1,140
checkpoints (every page and every ~20 ms of engine time) in 233–237 MB (`log_bytes` in
`raw/soundness-pass-a.jsonl.gz`); full-1000's 1,002 pages hold 244 MB. The 1 GB budget
never had to thin them.

### Soundness (L3)

`scripts/soundness.py` runs `scripts/incr_bench.py --verify` per document: a host session
settles the document, then applies random single-character edits (replace, insert or
delete a letter in the body) at 50 positions, compiling after each edit and after its
revert; every compile's `.pdf`, `.log`, `.aux`, `.out`, `.toc` and terminal output are
compared byte for byte with `flashtex-initex` run from scratch (preview mode, like the
host) on a copy of the directory as the compile found it. Seeds fixed
(`PYTHONHASHSEED=0`).

| pass (engine 64a710f6c) | documents | compiles | mismatches | converged | cold |
|---|---|---|---|---|---|
| A: 50 single-character edits + reverts | 82 fixtures + Hello, 120 pages, 953 pages | **8,500** | **0** | 500 | 386 |
| B: 10 sentence insertions + reverts | 82 fixtures + Hello, 120 pages, full-100 | **1,700** | **0** | 14 | 68 |

Logs were byte-identical in every compile (the P-T1 accounting normalisation was never
needed). Cold compiles are edits before the anchor (the `\documentclass`/preamble text of
small fixtures: the key is broken, as it should be), sessions whose previous compile
failed (`cannot restore`, below), and the first compile of each session. The 953-page
document converged in 14 of its 100 compiles, always inside the eighth copy of the edited
file, after the last read of the edited text. Earlier passes on intermediate engines
(`raw/soundness-s8*.txt`: 9,540 compiles, 0 mismatches, 9 beamer sessions stopped by the
restore error fixed in 64a710f6c) found nothing else.

### Reopen (§5.1, §1.2)

`scripts/reopen.py`: a host compiles the document until it settles and saves the anchor;
then five times (a) a fresh process opens it and ships page 1, (b) a process that first
ran `warm DIR` does the same (`raw/reopen-final.txt`, load 33–42):

| | Hello | 120 pages | full-1000 |
|---|---|---|---|
| snapshot on disk | 21.4 MB | 27.8 MB | 24.9 MB |
| **warm host: `open` to page 1, median (all)** | **13 ms** (11–21) | **78 ms** (51–161) | **70 ms** (50–332) |
| fresh process to page 1, median | 408 ms | 521 ms | 453 ms |
| warming a host (once per process) | 118 ms | 118 ms | 119 ms |

At load ~20 an hour earlier the same engine code gave warm 7 ms (Hello) and 52 ms
(full-1000, 51–53), fresh 150–180 ms. Opening at the `.aux` point rather than S₀ costs
LaTeX's begin-document work (full-1000 from S₀: 28 ms), bought for robustness: an `.aux`
changed since the save is an edit, not a cold run.

### Gates

P-T1/P-T2, lockstep, trip, etrip and drift ran on a04504216 (preview-mode parity on
0f4bd67cc); the later commits change only the host's restart logic (`incr.rs`) and tests,
which `flashtex-initex` does not run (`raw/`):

- **P-T1 82/82, P-T2 82/82** (`tools/parity/parity.py --tier fixtures --engine
  flashtex-initex`), per document identical to P4-L1's measurement with #1202; the
  non-gating accounting check differs on 82/82 exactly as there (1,359 candidate lines).
  With `FLASHTEX_PREVIEW=1`: **82/82, 82/82**, so preview mode changes nothing P-T1 or P-T2
  sees.
- **Lockstep 260/260**, accounting 0 differ.
- **trip, etrip** and web2rust's **drift** test pass; nothing under `src/generated/`
  changed.
- `cargo test --release -p flashtex-engine` (with `tests/incremental.rs`) and clippy
  `--all-targets` pass; `scripts/gate.sh pr` passes on a04504216 and on f87d3934f (the tip before this evidence; `raw/gate-pr-*.txt`).
- On the merged tree for #1215 (`agent/kabir-claude/p4-l1-image-state`, 5f8cd838f: P4-L1 +
  origin/main + the image-state fix): P-T1/P-T2 **83/83** (main added a fixture), lockstep
  260/260, `checkpoint-fixtures.py` **83/83** (selftest on the 7 beamer fixtures, which load
  images), `gate.sh pr` passes.

## What was built

### Fixed per-run cost and preview mode

- **Font map parsed once per process** (`pdftex/mapfile.rs`, `MapCache`): re-running the
  one-page *Hello* body from S₀ took 47 ms, 45 of them re-reading `pdftex.map`; now 1.3 ms.
- **pdfTeX's C-part state copy-on-write** (`pdftex/shared.rs`: `Shared<T>` over `Arc`,
  `ShardMap` of 64 shards): capturing it at a checkpoint went from 3.1 ms to about 1 µs.
  The image state is checkpointable (the lazily reopened PNG/JPEG/JBIG2/PDF handles clone),
  replacing P4-L1's stop-gap that refused checkpoints while an image was loaded.
- **Preview mode** (`pdftex::set_preview`, `FLASHTEX_PREVIEW=1` for the CLI; on in
  `iserve` unless `--no-preview`): streams are written with zlib level 0. Export is
  unchanged; P-T1/P-T2 are 82/82 either way.

### L2: checkpoints (`arena.rs`, `checkpoint.rs`)

- A checkpoint after every `\shipout` and after ~20 ms of engine time without one
  (`maybe_request_timed_checkpoint` from `input_ln`), both at `big_switch`.
- **1 KB chunks** (P4-L1 had 16 KB): about 1,900 dirty chunks, 1.9 MB, a page on the
  953-page document.
- **Per-page deltas.** The barrier still copies a whole chunk on its first write; the next
  checkpoint *seals* the log: each pre-image becomes a bit mask of the words that differ
  from the chunk then, plus those words (a tenth of them on the benchmark documents). The
  logs shrink about eightfold (full-100: 250 MB → 30 MB).
- **Restore, oldest entry first.** A word's value at a checkpoint is its oldest entry in
  the logs since; applying the logs from the target forward and writing each word once
  made a restore from the end of full-1000 to its middle (533 logs, 660k deltas over the
  same 2,078 chunks) 2 ms instead of 13. Views, the convergence diff and retention merges
  (older word wins) use the same logs.
- **Retention by budget** (`incr::thin`, default 1 GB): dense near the cursor, log-spaced
  beyond, spacing raised until the logs fit. With deltas no benchmark document needed it.

### L3: restart and converge (`incr.rs`, `iso.rs`, `statediff.rs`)

- **Journal.** Every file read (with the content of the user's files), every lookup, every
  output, and now every close of a user's input file with the bytes consumed. Checkpoints
  record their counts and every input stream's offset (line granularity: TeX's lookahead
  line counts as consumed).
- **Changes** are found by comparing each file with its journaled content (common prefix
  and suffix: one edit per file).
- **Restart point:** the newest checkpoint whose consumption of every changed file is
  before the edit: the file not yet opened, or open once at or before the edit's offset, and
  every earlier read of it, closed by then, stopped before the edit. Binary search.
- **The `.aux` point** (`Point::Aux`): the anchor of incremental runs is a checkpoint inside
  `\document`, just after LaTeX opens the `.aux` to test it exists, before reading it. The
  anchor's key covers the files read before it; the `.aux` is journaled like the document,
  so an `.aux` the previous run rewrote is an edit, not a broken key.
- **Convergence test** at each page after the edited one (backing off after three misses):
  (a) the same input positions (edit-shifted) and the same external effects (`\write18`,
  `\pdfelapsedtime`… are logged effects), (b) the old run reads no changed file later, and
  no file either run writes (up to its last checkpoint), (c) the same C-part state, and
  (d) the same word space, except words that cannot affect what follows: dead parts of
  stacks and buffers, cells free in both states, PDF file positions (relocated), dead
  scalars with the tex.web section that sets them before every read — and, where the runs
  allocated nodes at other addresses, a **structural comparison** (`iso.rs`): a bijection
  between the two states' node graphs from a complete root model (eqtb by type, the save
  stack, nest, input and parameter stacks, marks, sparse arrays, conditionals, alignments,
  hyphenation exceptions, pdfTeX's objects and static heads) under which every compared
  field is equal, every allocated cell is reached or free in both, and every word outside
  the model equal.
- **The jump.** On a match, the old run's state at that checkpoint is written over every
  differing chunk (so later restores see exactly the old run's states), then the redo log
  jumps to the old run's end and its later checkpoints are kept; output files and the
  terminal are spliced, PDF positions relocated. `\end{document}` always re-runs, from the
  old run's last checkpoint.

### L4: viewport first (`host_main.rs iserve`)

`compile N` stops at page N (the page is in the PDF and reported), `finish` continues the
same run; `pages` lists page frames (hashes of each page's PDF bytes) with the pages after
the cursor marked stale until the run passes them. The latency matrix measures the
edited page's ship time inside a full compile, which is when `compile N` returns.

### Reopen (`host.rs`, `incr::Session::save_s0/open_s0/warm_up`)

The anchor state is persisted (`save`) and reopened (`open PATH [N]`). A fresh process
pays kpathsea's initialisation and the font map (≈150–180 ms at the loads seen), so the
choice is a **pre-warmed host**: `warm DIR` compiles a throwaway one-page document once
(kpathsea kept across documents with the same program name), after which `open` is the
restore plus the pages up to the one asked for.

## Soundness bugs the tests found (all fixed, all in the history)

1. Output files a restart at S₀ reopened were not saved for the splice → saved from the
   old journal before the restore.
2. A file read twice was journaled once → every read is listed.
3. Relocation records outlived a cold run whose checkpoint ids restart → cleared.
4. The convergence jump left the new run's contents in chunks the old run did not write
   later, so a later restore of an old checkpoint (and the `\end{document}` re-run) could
   see a mixture — found by review, not by a mismatch → the old state is written over the
   differing chunks first.
5. A file `\input` twice (the 953-page document): the restart landed in the eighth copy
   and kept seven stale copies — the soundness run's long8 edits all differed → closes are
   journaled with their consumption.
6. A file written from edited text and read later converged before the read — found by a
   constructed document, DESIGN §5.3's barrier list → a file either run writes is a
   barrier.

`tests/incremental.rs` covers 5, 6 and the `.aux` point, and fails with each fix disabled.

## Deviations from DESIGN.md

- **§5.3 / D8: no running state hash.** Convergence compares the two states directly
  (word diff + the structural comparison) at the checkpoint being tested. It cannot collide
  and needs no hooks in `eq_define`; it costs 2–10 ms per test on 100–1,000 pages (the
  tests run after the edited page is out, and back off). An incremental hash would make
  each test cheaper but has to be proved to cover everything the comparison does.
- **PDF object and font numbers are not relocatable**: they must be equal. PDF file
  positions are relocated.
- **§5.2 restore**: sequential per worker range, oldest entry first; parallel above 4,096
  chunks. No shadow-paged keyframes: far-back restores are 2–4 ms on 1,000 pages.

## What remains for §1.2

1. **Pages that cost more than 16 ms to re-run.** The edited page costs the rest of the
   page before it plus itself; for the *full* documents that is 10–17 ms of CPU at the end
   of full-1000 and 14 ms for page 1. What would cut it: restart points inside the page
   before (a checkpoint between paragraphs; D7's ~20 ms timer rarely fires inside a 6 ms
   page, and a 2 ms timer measured no gain because the page builder still re-ships the whole
   page), and §5.6's per-page speedups (hyperref's per-page cost is the floor here).
2. **The compile after a label-moving reflow** reads a changed `.aux` and re-typesets from
   the `.aux` point to the edited page (0.27–3.9 s at pages 46–277). That is §5.5 L5: record
   which `\r@…` entries each page read and re-run only those pages, in the background, while
   the viewport shows the page typeset with the previous `.aux`.
3. **Latency under load.** Wall-clock p95s were 1.5–2× the CPU p95s whenever the machine was
   busy; the parallel restore in particular waits for cores. Not measured on an idle machine
   today.
4. **Reopen from a fresh process** is 150–450 ms (kpathsea initialisation and the font map);
   the ≤ 100 ms target holds only in a pre-warmed host, which the app would keep.
5. **Finding the edit** re-reads and compares the whole source (1.8 ms at 4 MB); an editor
   that sends its edit ranges removes it.
6. **Not implemented from §5.3's barrier list:** `\pdfelapsedtime` (a document that reads it
   is not reproducible from scratch either; the tests pin the clock).

## Reproducing

```
cargo build --release -p flashtex-engine
scripts/mkeng.sh NAME            # copies the binaries to /tmp/p4l2/NAME, builds fmt-NAME
python3 scripts/gen.py /tmp/p4l2/docs; mkdir /tmp/p4l2/src-full-1000 ...  # one dir per document
PYTHONHASHSEED=0 python3 scripts/soundness.py NAME -j 4 --trials 50 --dir OUT \
    --extra /tmp/p4l2/src-l8:long8:body.inc --extra /tmp/p4l2/src-lf:long-full:body.inc \
    --extra /tmp/p4l2/src-min:min
PYTHONHASHSEED=0 python3 scripts/soundness.py NAME -j 4 --trials 10 --kinds sentence ...
python3 scripts/matrix.py NAME OUTDIR && python3 scripts/matrix_sum.py OUTDIR
python3 scripts/ckcpu.py NAME /tmp/p4l2/src-l8 long8 3
python3 scripts/reopen.py NAME /tmp/p4l2/src-full-1000 full-1000 5
python3 scripts/barrier_test.py NAME     # write-then-read (tests/incremental.rs has it too)
cargo test --release -p flashtex-engine --test incremental
```

The drivers talk to `flashtex-host iserve`, whose commands are `compile [N]`, `finish`,
`pages`, `terminal`, `save PATH`, `open PATH [N]`, `warm DIR`, `quit` (one JSON line each).
