# FT-070 — engine performance baseline, 2026-09-13

Owner: FT-070 perf lane. Status: baseline, no optimization landed yet.
Produced by [`crates/perf-bench`](../../../crates/perf-bench/README.md) at
`main` `8465bbd0`.

- Machine-readable baseline: `crates/perf-bench/baselines/linux-x86_64-ryzen7-7800x3d.json`
- Human table: [`report.txt`](report.txt) — regenerate it from the JSON at any
  time with `flashtex-perf-bench --print <baseline.json>`, so the two cannot drift.

Reproduce:

```sh
cd crates/perf-bench
cargo build --release --locked
./target/release/flashtex-perf-bench --json report.json
```

No font environment is needed or wanted: the harness resolves
`apps/mac/Fonts` itself and builds the set with `FontSet::with_dirs`, which
probes nothing, so this box's TeX Live 2025 cannot supply substitute metrics.

## Host and method

AMD Ryzen 7 7800X3D, 16 logical CPUs, 32 GB, NixOS, `powersave` governor with
boost on, stock cargo `release` (opt-level 3, no LTO, codegen-units 16), no
`RUSTFLAGS`.

Each repetition runs in a child process of its own, so "cold" means cold: the
expander's thread-local stream cache, the font set's memoised faces and the
allocator all start empty, and peak RSS belongs to one document rather than to
the whole suite. Five repetitions per case (four above 300 KB, two above 1 MB,
where one render costs tens of seconds); 20 keystrokes per warm scenario (10
above 300 KB, 5 above 1 MB). Every metric carries its own `n`.

Everything reported is an order statistic — median, p25/p75, p95, MAD. The
table prints `±IQR`, `(p75-p25)/median` of `cold.render_ms`, as the run's own
noise estimate.

### Noise, honestly

**This box was not idle.** Other lanes were building throughout, and the report
records the peak 1-minute load average per case alongside every row. Within a
case the spread is small — `±IQR` is 1–6 % for most, though `hw1` (39 %) and
`letter` (42 %) were measured during the busiest stretch. *Between* runs at
different loads it is much worse: re-running `hw1` and `hw2` after the other
lanes finished moved `cold.render_ms` by −44 % with no code change at all,
while every output digest stayed identical.

Two consequences, both built into the gate:

- A 5 % tolerance is meaningless across a load gap that size. The gate warns
  and stops enforcing timings when the baseline's peak load differs from the
  current run's by more than about a factor of two, and when the host
  fingerprint, build profile or font configuration differ at all.
- Output identity does **not** depend on the machine — the fonts come from the
  repository — so digest comparison is enforced everywhere and unconditionally.
  That −44 % re-run is also the proof: 50 metrics moved, 0 of 60+ digests did.

For gating, record the baseline on a quiet machine and pin with `--pin`.

## Distance to the FT-070 targets

| target | budget | measured | ratio | met |
|---|---|---|---|---|
| warm keystroke at 500 KB | 2 ms | **306 ms** | 153x | no |
| warm keystroke at 2 MB | 10 ms | **1320 ms** | 132x | no |
| cold HW1 full compile | 10 ms | **6.6 ms** | 0.7x | **yes** |
| PDF export for HW1 | 10 ms | **55.9 ms** | 5.6x | no |
| steady memory at 2 MB | 100 MB | **4931 MiB** | 49x | no |

Warm targets are evaluated against the *slowest* scenario for that document: a
keystroke budget has to hold for every kind of keystroke, not just the cheapest.

`cold.render_ms` for HW1 is an empty `RenderCache` with the faces already
loaded — a full recompile in a running engine. The first render in a fresh
process, which also pays for loading the faces, is 54 ms (34 ms on a quiet
box); that is the number a user waits for when the engine has just started, and
font loading dominates it.

## Where the time goes

`cold.phase.*`, median ms. `typeset` is shaping, line breaking, math and page
building together — those four cannot be separated from outside the pipeline.
`typeset_notexts` is a control: the same typesetting with `Context::new`, i.e.
without the document sources.

| case | parse | adapt | typeset | typeset_notexts | assemble | v1 | json_v2 | cover |
|---|---|---|---|---|---|---|---|---|
| hw1 | 0.22 | 0.74 | 2.57 | 2.34 | 0.70 | 0.05 | 0.90 | 0.65 |
| hw2 | 0.22 | 0.81 | 2.23 | 1.93 | 0.57 | 0.05 | 0.80 | 1.03 |
| math-heavy (150 KB) | 7.9 | 15.7 | 42.9 | 41.8 | 25.5 | 2.2 | 26.1 | 0.78 |
| tikz-heavy (120 KB) | 4.7 | 9.2 | 92.8 | — | 11.8 | 1.0 | 15.6 | 0.93 |
| synthetic-500kb | 23.8 | 68.7 | **2924** | **174** | 131 | 13.2 | 134 | 0.97 |
| synthetic-2mb | 134 | 304 | **42704** | **1043** | 555 | 68.9 | 663 | 0.98 |

`cover` is `sum(parse..v1) / cold.render_ms`. Below 0.85 the product path does
work the decomposition does not replay — float scanning, TikZ scanning, extra
`\pageref`/contents passes.

## Finding 1 — a whole-document source scan per math formula

Typesetting dominates every large case, and almost all of it disappears when
the document sources are withheld:

| document | typeset with sources | without | ratio |
|---|---|---|---|
| 150 KB math-heavy | 42.9 ms | 41.8 ms | 1.0x |
| 500 KB synthetic | 2924 ms | 174 ms | **16.8x** |
| 2 MB synthetic | 42704 ms | 1043 ms | **41x** |

The ratio grows with document size, which is the signature of a per-formula
scan over the whole source. There is exactly one such scan, in
`Context::math_box` (`crates/render-pipeline/src/typeset.rs:930`), at line 941:

```rust
sink.amsfonts = self.texts.iter().any(|t| {
    crate::adapter::package_options(t, "amssymb").is_some()
        || crate::adapter::package_options(t, "amsfonts").is_some()
});
```

`math_box` runs once per math formula. `adapter::package_options`
(`adapter.rs:2003`) walks the source with `find_command(.., "usepackage")`
until it matches or reaches the end, so a document containing no
`\usepackage` at all costs a full scan of the whole file — and this asks twice,
for `amssymb` and for `amsfonts`. The 2 MB document has roughly 12 500
formulas, so this is on the order of 50 GB of scanning for one compile.

`math-heavy` shows a ratio of 1.0 because it declares `\usepackage{amssymb}`
in its preamble and the first search matches almost immediately; the synthetic
documents declare no packages, so every formula scans to end of file and finds
nothing. That is the whole 16.8x and 41x.

The question is about the **preamble** and the answer cannot change between
formulas, so it belongs in the `Context` once rather than in `math_box` every
time. On the 2 MB document this is roughly 95 % of the cold render.

One nearby line is *not* the problem, and is worth ruling out explicitly so the
fix does not start in the wrong place: `Context::math_fonts`
(`typeset.rs:588`) has a similar whole-document scan at line 620
(`self.texts.iter().any(|t| t.contains(a.command()))` over
`mathalpha::TEXT_ALPHABETS`), but `math_fonts` memoises on `self.math_fonts`
and returns early on every call after the first, so that scan runs once per
`Context`, not once per formula.

### Reproducing it

```sh
cd crates/perf-bench
cargo build --release --locked
./target/release/flashtex-perf-bench --only synthetic-2mb --only math-heavy \
    --runs 2 --steps 3 --caps v2 --no-export
```

Read the `typeset` and `typeset_notexts` columns of the phase table. The
harness measures the control by building the same document with
`Context::new` (no sources) immediately after the real `Context::with_texts`
build, so both see a warm allocator and the comparison is, if anything,
generous to the slow path.

Note for anyone reading older per-stage numbers:
`crates/render-pipeline/examples/stages.rs` builds its `Context` with
`Context::new`, i.e. **without** the sources. Its `typeset` figure is therefore
the `typeset_notexts` column here, not the one the worker pays, and the two
must not be compared.

## Finding 2 — PDF export is over budget, and cannot export two of the cases

HW1 export is 55.9 ms against a 10 ms budget (33.9 ms on a quiet box). A second
export of the same display list costs about the same, so this is not one-time
font subsetting; `write_pdf_exact` serialises the whole display list to a JSON
envelope and reads it back through the `v2` adapter on every export.

Two cases produce no export at all:

- `synthetic-2mb` (1507 pages): the envelope passes the pdf crate's 256 MiB
  limit and is refused. The harness declares the limit rather than spending a
  minute per repetition rediscovering it; raise `--export-max-pages` to
  reproduce.
- `tikz-heavy`: `item kind "path_stroke" is not supported by the exact route
  (glyph_run, rule and image only)` — the exact PDF route cannot export TikZ
  output at all.

## Finding 3 — memory

Steady-state RSS is 2625 MiB at 500 KB and 4931 MiB at 2 MB, against a 100 MB
target at 2 MB. `cold.render_nocache_ms` is within a few percent of
`cold.render_ms` on every case, so the reuse cache is not what costs the time —
but it is a large part of what costs the memory, and it is populated on the
first compile, before any edit can benefit from it.

## Corpus coverage on `main`

Twelve of fourteen cases measured. Two refused, correctly:

- `article-twocolumn` and `input-bibliography` use `\texttt`, and the
  typewriter faces and metrics (`lmmono*`, `ectt*`) are not in
  `apps/mac/Fonts`. The pipeline substitutes outlines and EC metrics, which
  changes line breaking — a different workload, not a slower one — so the
  harness produces no timing for them. They should start measuring once the
  typewriter metrics land.

Cases carrying a recorded degradation still produce timings, because the
degradation is deterministic and identical every run: `cv`, `letter`,
`math-sheet` and `unicode-accents` use class commands the compiler does not
model yet, and `unicode-accents` and `lecture-notes` contain characters with no
glyph in Latin Modern. Each is listed in the report's `notes`, so none of those
numbers can be read as a complete document.

## Gates at this commit

| gate | result |
|---|---|
| amsmath corpus | **59/59** within 0.5 bp |
| HW1 / HW2 | 3 pages each, 0 errors, **0 overfull** |
| `crates/render-pipeline` `cargo test --release` | 170 passed, 0 failed, 1 ignored |
| `crates/compiler` `cargo test --release` | 395 passed, 0 failed, 4 ignored |
| `crates/perf-bench` `cargo test --release` | 13 passed, 0 failed |

The amsmath gate needs a note. `oracle.py check --fonts apps/mac/Fonts` sets
both `FLASHTEX_FONT_DIRS` and `FLASHTEX_TFM_DIRS` to that one directory, which
matches neither layout the rooted required-metrics reader accepts — the TFMs
live under `texmf/` — so the pinned metrics do not load and every fixture is
laid out from OpenType advances: **0/59**, with no hint that fonts were the
cause. Pointing `--fonts` at a flat directory holding the `.otf` files, the
`.tfm` files and the GUST licence together gives **59/59**. A symlinked
`<exe>/texmf` does not help; the rooted reader does not follow it.

## Corrections made while building the harness

Each of these produced a plausible-looking number before it was caught. They
are recorded because trust in a benchmark should rest on what was checked, not
on the absence of visible problems.

- **A `parse` phase that read 0.00 ms.** It was derived as `parse_project`
  minus a separately timed `expand_project`. Those two overlap, and whichever
  runs second pays less for cold code and a cold allocator, so the difference
  was noise around zero. `parse` is now the single seam; `expand` is
  informational and is not summed.
- **A font gate that deleted most of the corpus.** Refusing every font *and*
  error diagnostic correctly refused the two fixtures with substituted
  metrics, but also refused `cv`, `letter`, `math-sheet` and
  `unicode-accents` for compiler coverage gaps and for CJK having no glyph in
  Latin Modern — six of ten real-world fixtures with no data. The line is now
  drawn at the configuration, not the document.
- **A control that crashed the suite.** `typeset_notexts` builds with
  `Context::new`, which leaves `texts` empty; the TikZ path then slices that
  empty string by the picture's byte range and panics
  (`vector-graphics/src/tikz/mod.rs:204`). It lost a whole 30-minute run. The
  control is now skipped for documents containing a picture, and a child that
  dies without reporting is recorded as not measured instead of aborting the
  suite. The panic is reachable from a public API — `examples/stages.rs` would
  hit it too — though not from the product path, which always passes sources.
- **Warm scenarios validated against the wrong bar.** Requiring the edited
  document to render clean silently skipped every warm scenario on the four
  fixtures that already carry errors; the bar is now "the edit made nothing
  worse". The prose anchor also landed mid-character in `unicode-accents` and
  panicked in `insert_str`; anchors are now snapped to a character boundary,
  with a test.
