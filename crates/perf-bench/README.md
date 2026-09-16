# flashtex-perf-bench — FT-070 engine performance harness

Measures the product render path over the committed real-world corpus and four
generated documents, and reports, per case:

- **cold** timings, split by phase (parse, adapt, typeset, display-list
  emission, runtime-v1, JSON) — so a regression can be attributed rather than
  guessed at;
- **warm single-keystroke** timings — the number the editor actually feels;
- **PDF export**;
- **peak and steady resident memory**;
- an **output digest** per case and per scenario, because byte-identical output
  is a hard gate and a faster run that changed a byte is a failed run.

`--baseline <report.json> --check` turns the same run into a CI gate that fails
on a regression above 5 % or on any digest change.

The committed baseline is `baselines/linux-x86_64-ryzen7-7800x3d.json`; the
numbers it holds, what they mean and what they say about the FT-070 targets are
written up in
[docs/evidence/perf-ft070-2026-09-13](../../docs/evidence/perf-ft070-2026-09-13/README.md).
`--print <report.json>` re-renders any committed report as the human table.

## Running it

```sh
cd crates/perf-bench
cargo build --release
./target/release/flashtex-perf-bench --json /tmp/report.json
```

Fonts need no setup: with no flags and no environment the harness uses
`<repo>/apps/mac/Fonts` and derives the metric directories from its `texmf`.
`--fonts DIR[:DIR]` and `--tfm-dirs DIR[:DIR]` override it;
`FLASHTEX_FONT_DIRS` / `FLASHTEX_TFM_DIRS` are honoured in between.

Useful invocations:

```sh
# smoke test (3 runs, 6 steps, v2 only)
./target/release/flashtex-perf-bench --quick --only hw1

# record a baseline
./target/release/flashtex-perf-bench --json baselines/<host>-<commit>.json

# gate a change against it (--require-same-host: enforce timings only when the
# baseline's host, build profile, font configuration and load match; a digest
# change still fails anywhere)
./target/release/flashtex-perf-bench --baseline baselines/<host>.json --check --require-same-host

# re-render a committed report, and compare two recorded runs without measuring
./target/release/flashtex-perf-bench --print baselines/<host>.json
./target/release/flashtex-perf-bench --print second-run.json --baseline baselines/<host>.json --quiet

# list the corpus / write the generated documents out for inspection
./target/release/flashtex-perf-bench --list
./target/release/flashtex-perf-bench --dump-corpus /tmp/corpus

# opt-in profile of one case (needs `perf`; the harness never requires it)
./target/release/flashtex-perf-bench --flamegraph /tmp/fg --flamegraph-case synthetic-500kb
```

## Three rules the harness enforces on itself

**No measurement without fonts.** Latin Modern and the pinned 12 pt TFM set
must load, and no render may raise any of the eleven font diagnostics in
`fontgate::FONT_CODES`. A substituted metric is not a slower run of the same
document — it is a different document, with different advances, different line
breaks and a different page count. Any violation aborts the run rather than
footnoting it. The font set is built with `FontSet::with_dirs`, which probes
nothing, so a host TeX Live installation can never supply the metrics.

**Cold means cold.** Each repetition runs in a child process of its own. The
caller-visible `RenderCache` is only one of the caches in play: the expander
keeps a thread-local token-stream cache, the font set memoises faces and
metrics, and the allocator keeps whatever the last document taught it. Only a
fresh process clears all of them, and only a fresh process can report a peak
RSS that belongs to one document instead of to the whole suite. The parent
spawns the children, so no spawn cost lands on a timed path.

**Medians, with the spread shown.** Nothing is reported from one sample. Every
metric carries `n`, min, p25, median, p75, p95, max, MAD and `rel_iqr`, and the
table prints `±IQR` — `(p75-p25)/median` — as the run's own noise estimate.

## Noise

This machine runs several build lanes at once and the numbers say so. Load
average is recorded before and after every case and the maximum goes in the
report as `load1_max`; a fixed integer-work loop is timed at the start and end
of the run and reported as `calibration_ns` with its drift. The gate warns when
the calibration moved more than 10 % from the baseline's, when the host
fingerprint differs, or when the build profile or font configuration differ —
in those cases the two runs measured different things and a delta between them
is not evidence.

The gate calls a metric regressed only when it exceeds **both** the percentage
tolerance **and** that metric's own measured spread in the baseline, and only
when it moved by more than 0.05 ms (2 MiB for memory). A gate that fires on any
5 % move on a shared machine gets switched off within a day, which is worse
than no gate.

For the quietest numbers, pin the children: `--pin 3`.

## The corpus

Ten real-world cases are discovered under `fixtures/real-world` with the same
rule `tools/real-world-corpus/run.py` uses — `main.tex` if present, else the
single `.tex`, with every `.tex` below the directory included so `\input`
resolves. That is the nine fixtures the corpus README documents, plus `hw2`.

Four generated cases are produced by committed code in `src/corpus.rs`, not
committed as blobs:

| case | size | what it is for |
|---|---|---|
| `synthetic-500kb` | 500 KB | scale. Byte-identical to the generator in `crates/compiler/src/bin/scaling_bench.rs` and to `docs/evidence/perf-2026-09-13/gen500.py`, so numbers cross-reference with the FT-065 evidence |
| `synthetic-2mb` | 2 MB | the same generator at the size the memory target is stated in |
| `math-heavy` | 150 KB | dense mathematics, almost no prose: puts math layout on top instead of the paragraph breaker |
| `tikz-heavy` | 120 KB | many `tikzpicture` environments: the vector-graphics path |

The generators are deterministic (SplitMix64, fixed seeds, no clock, no
environment). `--dump-corpus DIR` writes them out. The two shape-specific
documents each carry one command-free line of prose per block, because a
document with nowhere to type has no warm scenario and would measure half of
what it should.

## What the numbers mean

| metric | what it measures |
|---|---|
| `cold.first_render_ms` | the first render in a fresh process: cold caches **and** cold font faces. Opening a file in a freshly launched engine |
| `cold.render_ms` | empty `RenderCache`, faces already loaded. A full recompile in a running engine |
| `cold.render_nocache_ms` | the same render with no cache attached. The gap is what *populating* the reuse cache costs on the first compile, before any edit can benefit from it |
| `cold.phase.*_ms` | the decomposition below |
| `warm.<scenario>.<caps>_ms` | one keystroke through `protocol::handle_line`, against a warm cache — request JSON in, reply JSON out, exactly what the worker serves |
| `export.pdf_ms` | `write_pdf_exact`, including the one-time font-program read and subset |
| `export.pdf_warm_ms` | a second export of the same display list: the marginal cost |
| `mem.peak_rss_kb` / `mem.steady_rss_kb` | `VmHWM` / `VmRSS` at the end of the child. Linux only; `null` elsewhere, rather than a current-RSS reading standing in for a high-water mark |

Warm scenarios type one more character per step at a fixed anchor — in prose,
inside `$…$`, inside a display equation — plus one that alternately deletes and
restores a line. Anchors are found in the document, and a scenario whose edit
would introduce an error diagnostic is skipped with a note rather than timed:
otherwise it would be measuring error recovery, which is fast for the wrong
reason.

### The phase split, and its limits

The split is a **decomposition at the public seams**, not instrumentation
inside `render_cached`. `crates/render-pipeline` belongs to another lane, so
the harness adds no hooks to it. Three consequences are reported rather than
hidden:

- `coverage` = `sum(parse..v1) / cold.render_ms`. Below about 0.85 the product
  path is doing work the decomposition does not replay — float scanning, TikZ
  scanning, extra `\pageref`/contents passes — so attribute with care.
- `expand` is `expansion::expand_project` timed on its own. It **overlaps**
  `parse` rather than adding to it, and is excluded from the sum.
  An earlier version reported `parse − expand` as a "parse" phase; on HW1 that
  came out at 0.00 ms, not because parsing is free but because whichever of two
  measurements of overlapping work runs second pays less for cold code. A
  derived difference between overlapping measurements is not a phase.
- `typeset` cannot be split into shaping, line breaking, math and page building
  from outside the crate. Use `--flamegraph`, which also prints self-time folded
  into those crates, when that split is what you need.

`typeset_notexts` is a **control**, not a stage: the same typesetting with
`Context::new`, i.e. without the document sources. `render_cached` always uses
`Context::with_texts`. `crates/render-pipeline/examples/stages.rs` uses
`Context::new`, so its per-stage numbers describe a cheaper pipeline than the
one the worker runs, and the two must not be compared. Keeping the control here
makes the difference a measured quantity.

## Which compiler am I measuring?

`crates/render-pipeline` builds against pinned mirrors under
`crates/render-pipeline/vendor/`, so **a change under `crates/compiler` is
invisible to this harness until `vendor/compiler` is re-pinned.** When
measuring a compiler-side change, either re-pin first or measure the compiler's
own worker binary directly.

## Report format

`--json` writes `{ "schema_version": 1, "meta": {…}, "cases": [...],
"targets": [...] }`, matching the shape used by
`docs/evidence/real-world-corpus-*/report.json`. `meta` carries the host,
CPU model, governor and boost state, logical CPU count, OS, repository commit
and dirty flag, build profile with opt-level and rustflags, run and step
counts, capability sets, calibration, load, and the resolved font
configuration with a digest over the full search list. Two runs are comparable
when those agree.

## Targets

The harness evaluates the FT-070 acceptance targets on every run and prints how
far each is from its budget, met or not. Warm targets are evaluated against the
**slowest** scenario for that document: a keystroke budget has to hold for every
kind of keystroke, not just the cheapest one.
