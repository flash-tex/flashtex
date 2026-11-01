# P6-THROUGHPUT: cold full-compile speed against pdfTeX (2026-09-30)

Lane D2 `P6-THROUGHPUT` (#2 comment 5916223765; DESIGN.md §1 "parity beats speed, and
speed beats maintainability", §4.2 allowed optimisations, §5.6 L6, §12 P6). Branch
`agent/flashtex-2a/p6-throughput`, from origin/main 296c90197. Raw records are in
[`raw/`](raw/), the drivers in [`scripts/`](scripts/).

**Host.** This Mac: Apple M5 Pro (18 cores), 48 GB, macOS 26.6.2, rustc 1.98.1, TeX Live 2026
`pdftex` 1.40.29 as the oracle. It was shared with other lanes; every timed run waited
for the 1-minute load average to be below 3 (`bench.py --max-load 3`), and each table
gives the load range.

**Method.** `scripts/bench.py`, derived from the L6 lane's: every engine compiles each
document in its own directory until its `.aux` settles; then each of 7 repetitions runs
every engine once, in a rotating order. "Cold full compile" = a new process from the
format, one pass over the whole document, no resident host and no snapshot. Each run is
wrapped in `/usr/bin/time -l`: user+sys CPU, wall, instructions retired and cycles
elapsed. Documents: the P4-L2-L3 generator (`plain-N`: article, amsmath, geometry;
`full-N`: plus hyperref, siunitx, cleveref, xcolor, footnotes, cross-references) at
N = 10, 100, 300, 1,000 pages. Engines are `cargo build --release -p flashtex-engine`
(the default, bounds-checked build; no PGO): `base` = origin/main 296c90197
(`flashtex-initex` sha256 3a9ff3dc…), `new` = this branch at 7373464bf (c34b4e5d…; the later commits change comments and evidence only: `src/generated/` regenerates identically and the code section of a HEAD build is identical), `newnochk` = `new` with two measurement-only features (below).

## Result

**The new engine is faster than pdfTeX C at every size**, in CPU and in wall time, before and after this lane's changes (the gate held already at origin/main). This lane's changes save up to 3% of cycles and 3.5% of instructions against origin/main; the closest size is still plain-1000 at 0.95× CPU and 0.94× wall. `raw/bench-final.jsonl`, 7 interleaved runs per engine, medians (minimum in brackets), CPU = user+sys:

| document | pages | pdfTeX CPU / wall | base CPU / wall | **new** CPU / wall (min) | **new ÷ pdfTeX** CPU, wall | new vs base: cycles, instructions | load |
|---|---|---|---|---|---|---|---|
| plain-10 | 10 | 0.21 / 0.224 s | 0.14 / 0.156 s | **0.14 / 0.157 s** (0.14 / 0.151) | **0.67×, 0.70×** | +0.2%, -1.1% | 2.9-3.0 |
| full-10 | 11 | 0.41 / 0.417 s | 0.32 / 0.330 s | **0.31 / 0.323 s** (0.29 / 0.309) | **0.76×, 0.77×** | -2.2%, -2.9% | 2.9-3.0 |
| plain-100 | 100 | 0.29 / 0.308 s | 0.22 / 0.238 s | **0.23 / 0.241 s** (0.22 / 0.232) | **0.79×, 0.78×** | +1.2%, -1.5% | 2.9-2.9 |
| full-100 | 101 | 0.78 / 0.794 s | 0.53 / 0.549 s | **0.53 / 0.551 s** (0.51 / 0.522) | **0.68×, 0.69×** | -0.4%, -3.2% | 2.7-2.9 |
| plain-300 | 301 | 0.45 / 0.461 s | 0.38 / 0.395 s | **0.37 / 0.389 s** (0.37 / 0.386) | **0.82×, 0.84×** | -1.5%, -2.2% | 2.6-2.8 |
| full-300 | 299 | 1.67 / 1.690 s | 1.04 / 1.061 s | **1.03 / 1.096 s** (1.02 / 1.040) | **0.62×, 0.65×** | -1.3%, -3.4% | 2.6-2.9 |
| plain-1000 | 1001 | 1.01 / 1.028 s | 0.99 / 0.998 s | **0.96 / 0.969 s** (0.95 / 0.960) | **0.95×, 0.94×** | -2.9%, -2.5% | 2.5-2.8 |
| full-1000 | 1002 | 4.81 / 4.909 s | 2.90 / 2.963 s | **2.86 / 2.870 s** (2.77 / 2.795) | **0.59×, 0.58×** | -2.1%, -3.5% | 2.5-3.0 |

Cycles and instructions are the Apple PMU's counts for the process; instructions do not move with load. The cycle differences to base on plain-10, plain-100 and full-100 (−0.4% to +1.2%) are within run-to-run noise. An earlier pass of origin/main against pdfTeX alone (`raw/bench-base.jsonl`, started 17:26Z, same gate) measured each engine's cycles within 3% of this one, except full-10 (6%); its CPU seconds differ by up to 13% on the 10-page documents, where one run is 0.1-0.3 s.

## Profile (before)

`sample` at 1 ms, `scripts/prof.sh` + the L6 lane's `sampletop.py`; self time.

| function | plain-1000 | full-1000 | what it is |
|---|---|---|---|
| `get_next` | 12.8% | 25.6% | TeX's token reader, once per token |
| zlib (`longest_match`, `deflate_slow`, …) | ~16% | ~5% | TeX Live's zlib 1.3.2, as pdfTeX uses it |
| `macro_call` | 4.7% | 11.9% | macro argument matching |
| `get_avail`, `get_node`, `free_node`, `flush_node_list` | ~12% | ~9% | node and token allocation |
| `end_token_list` | 2.9% | 5.8% | leaving a token list |
| `divide_scaled` | 2.2% | 1.0% | PDF coordinates |

`get_next` compiled to one routine of 1,269 instructions with its external-file part
(reading characters, scanning control-sequence names, next line) and, inlined, the
read-set hooks of `changes/readset.ch`; every call saves seven register pairs and
materialises a dozen constants before the token-list path runs (`raw/asm-base.txt`,
`scripts/asmcalls.sh`). With the changes below it is 246 instructions
(`raw/asm-new.txt`). It still saves seven pairs: they hold constants that the compiler
hoists out of the `restart` loop, which the file part's removal does not change (moving
the token-list part out as well removes them, and is slower; see below).

## Changes

All through WEB change files or attributes; `src/generated/` is regenerated, never
edited. Array reads stay bounds-checked; no unchecked read was added.

1. **`changes/throughput.ch` [1]: `get_next`'s external-file part out of line** as
   `get_next_file`, same statements in the same order; its `goto restart`, `return` and
   fall-through are its result (0, 1, 2). Proof of identity in the change file.
2. **`changes/throughput.ch` [2]: `divide_scaled` as one 64-bit division** (DESIGN.md
   §4.2 names this optimisation). The digit loop computes the truncating quotient and
   remainder of `s*10^dd` by `m`, wrapped to 32 bits; the change computes them exactly in
   64 bits and wraps where the result is stored. Proof in the change file;
   `tests/divide_scaled.rs` compares result and `scaled_out` with a transcription of
   pdftex.web's loop on edge values, every small case and 3,000,000 random arguments
   (and fails when the rounding rule is changed).
3. **`src/readset.rs`: `flashtex_cs_read`, `flashtex_id_read` `#[cold]
   #[inline(never)]`.** Only the resident host turns read-sets on. Separate commit,
   flagged for the L5 owners.

The step-by-step effect (`raw/ab1-ungated.jsonl`, 5 runs without the load gate, load 4-5; cycles against base): [2] alone −2.2% plain-300, −1.0% full-300; with [1] −1.9% / −2.0%; with [3] too −2.0% / −2.3%.

Reuse (DESIGN.md §1): nothing new is built here; the zlib alternatives were evaluated by the L6 lane (none byte-identical).

Measured and not adopted:

- **`get_next`'s token-list part out of line too** (`get_next` becomes 92 instructions
  saving 3 register pairs): 0.1% fewer instructions but 1-3% more cycles on full-300
  (`raw/ab2-ungated.jsonl`: 5 runs without the load gate, load 4.8-5.1). The extra call costs more than the saved registers.
- **A 64 KiB PDF write buffer** (`src/system.rs`, `b_open_out`): the 5.4% `write` share
  of the first plain-1000 profile was disk contention from a concurrent run; on a quiet
  profile `write` is 0.6% with the 8 KiB buffer and 0.7% with 64 KiB. pdfTeX flushes
  after every stream (writezip.c) and seeks back for `/Length`, and so does the port, so
  the buffer size barely changes the number of writes.

## What the safety costs (measurement only)

`newnochk` = `new` built with the benchmarking features `unchecked-reads` and
`bench-no-barrier` (reads without bounds checks, writes without the checkpoint barrier;
both never shipped, `src/ix.rs`, `src/arena.rs`):

| document | new: cycles, instructions | `newnochk` | difference |
|---|---|---|---|
| plain-10 | 0.65G, 2.44G | 0.64G, 2.29G | -2.3% cycles, -6.4% instructions |
| full-10 | 1.36G, 5.93G | 1.23G, 5.13G | -9.7% cycles, -13.5% instructions |
| plain-100 | 1.01G, 3.98G | 0.96G, 3.48G | -5.2% cycles, -12.7% instructions |
| full-100 | 2.34G, 10.83G | 2.10G, 8.99G | -10.3% cycles, -17.0% instructions |
| plain-300 | 1.69G, 7.38G | 1.56G, 6.13G | -8.0% cycles, -16.8% instructions |
| full-300 | 4.54G, 21.74G | 4.03G, 17.57G | -11.3% cycles, -19.2% instructions |
| plain-1000 | 4.22G, 19.29G | 3.82G, 15.42G | -9.6% cycles, -20.1% instructions |
| full-1000 | 12.41G, 60.24G | 10.93G, 47.84G | -11.9% cycles, -20.6% instructions |

Together, the bounds checks on reads and the checkpoint write barrier cost 8-12% of the cycles from 300 pages up (2-10% below) and 6-21% of the instructions. They are the largest remaining item that is not TeX's own work. Both stay: the Commander decided in #1241 that reads stay checked, and the barrier is what makes checkpoints correct (P4-MEMORY owns `src/arena.rs`). A cheaper check against a length held in a register belongs to the arena's owner.

## Gates

At 7373464bf (the benchmarked binary is identical, sha256 c34b4e5d…):

- **Byte identity**, `scripts/identity.py base new -j 2`: 80 parity fixtures
  (`fixtures/real-world`, `fixtures/divergence-probes`) and the 8 benchmark documents,
  each compiled to a settled `.aux`: PDF and log byte-identical, **0 of 88 differ**
  (`raw/identity.txt`).
- **P-T1 83/83, P-T2 83/83** on the parity fixtures (`-j 2`), no fixture below its
  baseline (`raw/parity-fixtures-new/`).
- **Lockstep 1145/1145 equal, 0 differ** (`raw/lockstep-new.txt`). Its accounting line
  (`1410-tracingstats differs (memory usage)`) is the same on origin/main's build.
- **trip** pass (`raw/trip.txt`), **etrip** pass with `throughput.ch` applied
  (`web2rust-etrip.args` lists it; `raw/etrip.txt`), web2rust **drift** pass
  (`raw/drift.txt`), `cargo test --release -p flashtex-engine` pass
  (`raw/engine-tests.txt`), including `tests/divide_scaled.rs`.
- `scripts/gate.sh pr`: rustfmt, clippy, tests, parity baseline pass; the licence
  boundary check fails before it checks anything here (`check-license-boundary.sh: line
  387: unexpected EOF`, the macOS `/bin/bash` 3.2 parse), as on origin/main.

## Reproducing

```sh
S=docs/evidence/p6-throughput-2026-09-30/scripts
python3 docs/evidence/p4-l2-l3-2026-09-29/scripts/gen.py ~/flashtex-wt/d2/docs
CARGO_TARGET_DIR=~/flashtex-wt/target-d2 cargo build --release -p flashtex-engine
bash $S/mkeng.sh new ~/flashtex-wt/target-d2          # engine + its pdflatex format
python3 $S/bench.py plain-10 full-10 plain-100 full-100 plain-300 full-300 \
  plain-1000 full-1000 --engines pdflatex,base,new --reps 7
bash $S/profcmp.sh plain-1000 3 base new             # sample profiles
bash $S/asmcalls.sh ~/flashtex-wt/d2/eng/new/flashtex-initex get_next get_next_file
python3 $S/identity.py base new -j 2                 # outputs vs origin/main's
bash $S/gates.sh new                                 # parity, lockstep, trip, etrip, drift, tests
```
