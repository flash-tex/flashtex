# P4-L1 foundation: one flat word space, checkpoints, the resident engine and S₀ (2026-09-29)

Lane P4-L1-FOUNDATION (DESIGN.md §4.2, §5.1 L1, §5.2 L2, §12 P4). Branch
`agent/kabir-claude/p4-l1-foundation`, PR #1215 (stacked on #1198). Raw output in
[`raw/`](raw/), the measuring scripts in [`scripts/`](scripts/) (they expect the
documents in `/tmp/p4l1/docs` and engines in `/tmp/p4l1/<name>/`; see `scripts/final.sh`).

**Host:** mac-m5pro-kabir, Apple M5 Pro, 24 GiB, macOS 26.6.2, rustc 1.98.0, release
builds. The machine was shared with other agents throughout: 1-minute load average
7–24 during the final set (`raw/environment.txt`), up to 90 during earlier runs. Every
engine-versus-engine number below is therefore **paired**: all arms start at the same
moment on separate cores, so background load hits them alike, and the ratio is taken per
repetition (`scripts/timeit3.py`); cycles and instructions come from `/usr/bin/time -l`.

**Documents** (the ones DESIGN.md Appendix B was measured on): *Hello* is
`\documentclass{article}\begin{document}Hello\end{document}`; *120 pages* is
`long-full.tex` (article, geometry, amsmath/amssymb/amsthm/mathtools, graphicx, booktabs,
xcolor, tikz, siunitx, hyperref, cleveref) with `\input{body.inc}`; *953 pages* is
`long8.tex`, the same preamble with `body.inc` input eight times.

## Results

### The measured table

| | Hello | 120 pages | 953 pages |
|---|---|---|---|
| cold full run (host, from the format) | 71 ms | 0.84 s | 4.7–5.1 s |
| S₀ taken at (preamble) | 20 ms | 238 ms | 242 ms |
| **S₀ restore** (`restore_discard`) | **2.7 ms** | **2.9 ms** | **3.1 ms** |
| S₀ key check (stat + re-lookup) | 0.2 ms | 4.5 ms | 5.9 ms |
| **body re-run from S₀** (to the end of the job) | 47 ms | 0.61 s | 4.3–4.6 s |
| first page after the restore | 49 ms | 63 ms | 66 ms |
| undo log held by S₀ after a body run | 4.0 MB | 9.8 MB | 10.2 MB |
| **S₀ on disk** | **22.2 MB** | **28.8 MB** | **29.0 MB** |
| save S₀ | 19 ms | 33 ms | 21 ms |
| **reopen S₀ in a fresh process** (`open_s0` total) | **67 ms** | **67 ms** | **65 ms** |
| … of which `system::configure` (texmf.cnf, kpathsea) | 63 ms | 57 ms | 54 ms |
| … of which reading the header, loading 1.3–1.8 k chunks, host state | 4 ms | 6 ms | 5 ms |
| … of which the key check | 0.2 ms | 4.9 ms | 5.7 ms |
| reopen to first page (fresh process) | 118 ms | 130 ms | 125 ms |
| **write barrier, cycles vs the current engine** (paired, median [IQR]) | +0.6 % [−0.8, +0.9] | **+2.6 % [+1.9, +2.9]** | **+2.6 % [+2.3, +2.9]** |
| … the same layout without the barrier | −0.5 % | −0.4 % | −0.3 % |
| … instructions retired | +1.9 % | +9.0 % | +10.2 % |

`raw/host-*.json` (medians of 3–5 compiles and 3–5 fresh-process reopens),
`raw/barrier-*.jsonl` (30 / 16 / 8 paired repetitions), `raw/layout.json`.

**The 3 % gate (DESIGN.md §5.2, left open by the snapshot benchmark) is met, narrowly.**
On the real engine the barrier costs **+2.6 %** of cycles on 120 and 953 pages in the final
paired set (upper quartile 2.9 %). It is not comfortably under: earlier paired sets on this
machine gave +2.75 % [1.8, 3.8] and +3.04 % [2.5, 3.4] at load 10–15, and +4.4 % at load
~90; sequential wall-clock runs at load 7 gave +1.8 % (120 pages) and +3.7 % (953 pages,
3 runs) (`raw/cli-wall-*.txt`). The flat layout itself costs nothing (the `bench-no-barrier`
build is 0.3–0.5 % *faster* than today's engine), so the whole cost is the barrier. See
*The barrier* for why it cannot get much cheaper and what would.

### Correctness

- **P-T1 unchanged.** Parity fixtures (`tools/parity --engine`, fixtures tier): **P-T1 75/82,
  P-T2 75/82, identical per document to the #1198 base** (`raw/parity-fixtures*.txt`); the
  7 beamer fixtures need image inclusion (#1202). With #1202 merged locally (not pushed):
  **P-T1 82/82, P-T2 82/82**. Lockstep **260/260**, also with #1202 merged locally
  (`raw/lockstep.txt`). trip, etrip and web2rust's drift test pass. The pdflatex format is
  byte-identical to the base engine's.
- **Checkpoint tests on the fixtures** (`crates/flashtex-engine/scripts/checkpoint-fixtures.py`,
  `raw/checkpoint-fixtures.jsonl`): on all 82 fixtures, each first run to convergence,
  - `selftest`: a run with a checkpoint at S₀ and after every shipout; restore each (up to 6
    per document), check the state is bit-identical to the one recorded there, run to the
    end, and check the state at **every later checkpoint**, the final state, every file in the
    directory and the terminal against the uninterrupted run; plus `redo_to` and
    `restore_discard` — **75 pass**, 7 n/a (the beamer fixtures stop with a fatal
    image-inclusion error before S₀ on this base; with #1202 they run, and the host refuses
    to checkpoint them, see *Limits*);
  - from S₀: a compile that restores S₀ writes the same `.pdf`, `.log` and `.aux` as
    `flashtex-initex`'s full run — **82/82** (75 of them actually from S₀).
- **Edits.** After edits to `body.inc`, the compile from S₀ is byte-identical (`.pdf`,
  `.log`, `.aux`) to a full `flashtex-initex` run on the edited source, on all three documents
  (`scripts/l1equiv.sh`); `crates/flashtex-engine/tests/checkpoint.rs` checks the same on a
  hyperref document with labels and footnotes, and that a persisted S₀ reopened in a fresh
  process writes the same files.
- Unit tests of the chain (`arena.rs`): every retained checkpoint restores exactly, the
  convergence jump reproduces the latest state, merged logs keep every kept checkpoint
  exact, straddling 24-byte records are saved whole, the parallel restore is exact.

## What was built

### One flat word space (`crates/flashtex-engine/src/arena.rs`, emitted by `tools/web2rust`)

Every array global is a region of one zero-filled anonymous mapping: **133 arrays in
445 MB of address space, 27,169 chunks of 16 KB** (`raw/layout.json`). The largest are
reservations for web2c's growable arrays at pdfTeX's `sup_*` limits (`obj_tab` 201 MB,
`pdf_mem` 40 MB, `pdf_os_buf` 20 MB), and `font_info` 64 MB, `mem` 40 MB, `str_pool` 25 MB;
untouched room costs address space, not memory. The **5,192 bytes of scalar globals** are
spilled into the first region at every checkpoint (`Globals::visit_scalars`, generated); the
file globals are listed by `Globals::visit_files`. web2rust emits `Arr<T>` for every array of
plain data, `alloc_len`/`resize_len` for web2c's `xmalloc_array`/`xrealloc_array`, fixed
Rust arrays for array type aliases (`char_used_array`), and `--arena-cap` for the four
arrays pdfTeX grows. `src/generated/` stays generated (drift test).

- **Reads** are plain loads (`Index`, `Deref`), as with the `Vec`s before.
- **The barrier** is `IndexMut`: `flags[addr >> 14]` — one byte per chunk, addressed from
  the element's own address through a pointer biased by the space's base — and, on a zero
  byte, a cold call that copies the chunk into the open undo log. Four instructions
  (`ldr`, `lsr`, `ldrb`, `cbz`); elements whose size is not a power of two (the 24-byte
  input-stack and object-table records) test both chunks they may straddle.
- **Checkpoint** (`Arena::checkpoint`): seal the open log, clear the flags. Before any
  checkpoint the flags only note which chunks were ever written, so hashing and persisting
  skip the rest.
- **Restore** walks the logs back to the target (each chunk's oldest pre-image), capturing the
  live chunks into a redo log (`restore_branch`) or not (`restore_discard`); **`converge`**
  jumps back to the old run's latest state and re-attaches its later checkpoints; **`retain`**
  merges logs. Chunks come from a slab of 1 MiB blocks; restores of ≥ 256 chunks run on up
  to 8 threads. This is the snapshot benchmark's (b3) chain.

### Checkpoint API (`src/checkpoint.rs`)

`Globals::checkpoint() -> CheckpointId`, `restore(id)` (keeps the run it leaves),
`restore_discard(id)`, `redo_to(id)` (the §5.3 jump: the live state equals the old run's at
`id`; output streams get the old run's bytes from their length at `id`), `resume_to_end()`,
`retain_checkpoints`, `state_hash()`. Each checkpoint records an **`ExtRecord`** of what lives
outside the space:

| external state | where it is |
|---|---|
| open input files and `\read` streams | path + byte offset of the next unread line + the Pascal lookahead (`system::FileSnap`); reopened and sought on restore |
| `\write` streams, the log, the PDF and DVI files | path + length; cut back to it on restore (a persisted S₀ carries the bytes) |
| the PDF output buffer, object and destination tables, object offsets | in the space (`pdf_op_buf`, `pdf_os_buf`, `obj_tab`, `dest_names`, `pdf_gone`, …) |
| pdfTeX's C-part state (font map, encodings, colour stacks, AVL trees, VF packets, subset tags, start time) | `pdftex::CState`, cloned; each module registers its state (`Clone` + `codec_struct!`); the idle zlib stream is not carried (a new stream re-initialises: same bytes) |
| the terminal | captured by the host; length |
| external effects (`\write18`, pipes, mktex) | count; any before S₀ makes S₀ unusable |
| kpathsea caches | not state: a cache over the file system; the read-set's lookups validate it |
| the random seed | in the space (`random_seed`, `randoms`); seeded from the pinned clock |
| the pinned date | the key; `pdftex::utils::pin_clock` pins `\time`/`\day`/`\month`/`\year`, the creation date and the seed per session |

### Checkpoint points (`changes/checkpoint.ch`)

A checkpoint can only be taken where nothing is on the Rust call stack that the state does
not describe: **`big_switch`** in `main_control`, between two commands. There the engine
calls `flashtex_checkpoint_hook` whenever `ckpt_request` is nonzero, and a restored run
re-enters `main_control` there (`ckpt_resuming` skips `\everyjob`). `ship_out` sets the
request after each page when asked (L2). Nothing the change adds alters what the program
computes; the hooks are a compare each in `pop_input`, `macro_call` and `big_switch`.

### S₀ and the resident engine (`src/host.rs`, `flashtex-host`)

**The point.** The host names the control sequence `\document`. When `macro_call` pushes its
body, the input level is remembered; when `pop_input` goes below it, the body — and
everything it expanded to — has been consumed, and the **next `big_switch` takes S₀**. It
uses no LaTeX-specific knowledge beyond the name. It is the earliest safe point after
`\document`: LaTeX's `\document` ends with `\ignorespaces`, which may already expand the first
body material (in the benchmark documents, `\input{body.inc}` is on the `\begin{document}`
line), so S₀ can sit a few tokens into the body; the key then covers those lines too.

**Why it is safe.** S₀ is a checkpoint like any other — all engine state at `big_switch` —
so restoring it and running on *is* the uninterrupted run, provided every input consumed by
then is unchanged. **The key** (`host::Key`) says what that was:
- every file opened, by content hash **at open** (so the `.aux` is the one `\document` read,
  before it rewrites it), with its stat signature as a fast path;
- every lookup, found or not (a file that appears later changes a run as surely as an edit);
- for each input file still open, the bytes consumed so far — **whole lines**, since TeX
  holds the current line in its buffer (DESIGN §5.2: line granularity);
- the files the preamble wrote and closed (written back before a restore);
- the pinned clock, `SOURCE_DATE_EPOCH`/`FORCE_SOURCE_DATE`, the first line, the job name,
  the engine build (a hash of the executable);
- any external effect before S₀ (then S₀ is not used).

A compile checks the key (stat, then hash only what changed, then re-run every lookup); if
it holds, it restores S₀ and runs the body; else it runs in full and takes a new S₀. The
first compile of a new document runs twice in full (the second time because its `.aux`
appeared), as pdflatex does.

**Persisted S₀** (`save_s0` / `open_s0`): a header (key, host record, the output files'
prefixes and the captured terminal, the list of nonzero chunks) and those chunks, 16 KB
aligned; opened by `mmap` and copied into a fresh space. `flashtex-host serve|bench|open|
selftest|layout` drives it; `serve` is a line protocol (`compile`, `save PATH`, `quit`) for the
editor.

## The barrier: where the 2.6 % goes, and what would reduce it

- A 120-page run makes **304 M element writes** (`bench-count-writes`): `mem` 81.5 %,
  `str_pool` 4.0 %, the small arrays next to the scalars 3.5 %, `save_stack` 2.9 %,
  `str_start` 1.8 %, the input and parameter stacks 1.1 % each, `eqtb` 1.1 %. Each pays the
  barrier's four instructions (+9–10 % instructions retired), which the core overlaps down
  to about a third of a cycle per write (+2.6 % cycles).
- Tried: an unconditional flag *store* instead of test-and-branch (what a post-image log
  would need): no better, +3.1 % (`raw/barrier-long-full.jsonl`, arm `store`). A peephole
  that skips the barrier on a write to the element the array wrote last could remove at
  most 7.5 % of the barriers (23 M of 304 M; 40 % hit the same *chunk*, but only the same
  element is provably safe).
- **Proposed if the margin has to grow:** (1) route array writes in generated code through
  one `Globals` field holding the flag base, so LLVM keeps it in a register across a routine
  (it cannot CSE 133 per-array copies) — one of the four instructions; (2) hardware dirty
  tracking (`mprotect` + a fault handler) for `mem` alone: no standing cost, ~5 µs per chunk
  per interval (the snapshot benchmark's copy-fault cost) — it wins while checkpoints are
  rare (L1: ~600 chunks written per body run, a 9.8 MB undo log) and loses ~10× at one
  checkpoint per page (~235 `mem` chunks per page), so it only fits if L2's intervals grow;
  (3) bulk (`slice_mut`) writes for loops that fill arrays element by element (the format
  undump, `input_ln`, the PDF buffers).

## What L2 and L3 need next (measured here)

A checkpoint after every shipout works (the selftests above take them) but is not yet
affordable (`raw/every-shipout-stats-*.json`):

| per page, checkpoint after every shipout | 120 pages | 953 pages |
|---|---|---|
| host state: pdfTeX's C-part state | **3.1 ms** | **3.3 ms** |
| host state: file streams | 29 µs | 29 µs |
| scalar spill + seal | 21 µs | 22 µs |
| chunks written per page (16 KB) | **429** (mem 235, eqtb 169) | **429** (mem 238, eqtb 170) |
| undo memory, no retention | 848 MB | 6.7 GB |
| run time | 1.33 s (L1: 0.84) | 9.1 s (L1: 4.7) |

1. **The C-part state copy dominates** (99 % of the checkpoint cost): the font map, loaded
   at the first shipout, is thousands of `FmEntry`s and three `BTreeMap`s cloned whole. It
   needs structural sharing (entries and trees behind `Arc`, copy-on-write on the rare
   mutation) — target ≪ 0.1 ms.
2. **Pages dirty ~6.9 MB, not the ~58 KB the benchmark modelled**: `mem` writes spread over
   ~240 chunks per page (fragmented free list) and `eqtb` over 170 of its 307 chunks (the
   output routine's local assignments land at hash positions). Retention and compaction
   (`retain` exists) must be budget-driven from the first L2 commit, and `eqtb` wants a finer
   granularity (1–4 KB sub-chunks or word-level undo) — it is 40 % of the volume for 1 % of
   the writes.
3. **A structural state hash** (§5.3: address-independent) for convergence; today only the
   whole-space byte hash the tests use (`state_hash`) exists, and it matches only when the
   re-run allocated exactly as the old run did.
4. **Per-page external-effect logs** for `\write`, PDF objects and `\pdfsavepos`: `redo_to`
   splices open output streams only; a file closed between two checkpoints keeps what was
   last written to it.
5. **Images (#1202):** register the image table (reopen libpng/JPEG/xpdf handles on
   restore). Until then the interim registration refuses a checkpoint while an image is
   loaded, so those documents run in full (verified on the beamer fixtures with #1202
   merged locally: outputs equal, no S₀).
6. **Restarts** (L3) choose the newest checkpoint before the edited line from the
   `ExtRecord` offsets, which are already per file at line granularity.

## Limits of L1 as built

- L1 saves the preamble only (0.24 s of 0.84 s on 120 pages; 0.24 of 4.7 s on 953 pages). The
  body re-runs in full; L2/L3 are the lever.
- Even *Hello* re-runs 47 ms from S₀: `\end{document}` reads the font map (once per run
  today; DESIGN §4.2 makes it once per process) and embeds the fonts.
- A reopened S₀ reaches its first page in 118–130 ms, of which 54–63 ms is `system::configure`
  (texmf.cnf and kpathsea's `ls-R`), paid by every fresh process; a resident host pays it once.
  The ≤ 100 ms reopen target (§1.2) needs that start-up warm or cached.
- S₀ is refused (the host runs in full) when the preamble ran an external command, holds a
  pipe, or — with #1202 — has an image loaded.
- `system::make_pdftex_banner` kept its once-made flag in a process-wide static (C's
  once-per-process is once-per-run); a resident process makes many runs, so the flag is now the
  engine's own `pdftex_banner`. Found by the S₀ equivalence check (a second run lost
  `/PTEX.Fullbanner`).
