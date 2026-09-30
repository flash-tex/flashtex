# Fuzz harness (t6-fuzz)

Structure-aware differential fuzzer: candidate engine vs oracle.
MIT-licensed, Python 3 standard library only, nothing to install.

## Run

```sh
python3 tools/fuzz/run.py --candidate BIN --oracle BIN --seeds DIR \
  --out DIR --iterations N --seed S --timeout SEC
```

Each iteration picks a seed case (or generates a fresh input), applies one
mutation from `gen.py`, and runs both engines through the lockstep
`capture()` (`-ini -etex`, same args and pinned environment). Every
generated input starts with `\input prelude`, the lockstep case convention.

`FLASHTEX_POOL` and `FLASHTEX_FORMATS` from the harness environment are
passed through to the candidate's `capture()` call only, never the oracle.

## Classes

- `equal`: same return code, same compared log.
- `diverge`: return codes or compared logs differ.
- `candidate-crash`: candidate died by signal (returncode < 0) or exited
  101 (Rust panic exit). Transcript text never decides a crash: a document
  containing the words `panicked at` (e.g. `\message{panicked at}`) with a
  normal return code is not a crash.
- `oracle-crash`: oracle died by signal (returncode < 0) only. Exit 101
  from the oracle is not a crash.
- `both-crash`: candidate AND oracle both crashed (pdfTeX itself
  crashes, so this is NOT an engine-diff; known-benign for nightly).
- `both-fail`: both engines non-zero without crashing (not interesting).
- `both-hang`: both engines exceeded `--timeout` (known-benign).
- `timeout`: exactly one engine exceeded `--timeout`.
- `output-flood`: one engine was killed by SIGXFSZ (return code -25 or
  152): it wrote past the file-size cap (a finding).
- `both-flood`: both engines flooded (known-benign, like `both-hang`).

## Output cap

Every entry point (`run.py`, `docgen.py`, the five parsers,
`minimize.py`) calls `run.apply_fsize_limit()` first: `RLIMIT_FSIZE`
is set to `FUZZ_FSIZE_LIMIT_BYTES` (default 64 MiB), inherited by all
engine children, so a runaway engine is killed by SIGXFSZ instead of
writing a multi-gigabyte log. A log over the cap is a flood finding
(`output-flood` / `both-flood`), never a comparison: `classify()` and
`first_diff()` run on the FULL logs, and only what is written into
artifacts and JSON is truncated to head and tail (`LOG_MAX_BYTES`,
64 MiB). The cap is what keeps memory bounded: one log file can never
grow past `FUZZ_FSIZE_LIMIT_BYTES`, so no transcript read into memory
can exceed it. Parser outputs go to a temp file and only the head and
tail are read back (`run_capped`/`read_capped`), and `crash_stderr()`
reads stderr through a pipe keeping only the last 64 KiB, killing the
process group after 64 MiB in total.

## Mutation weights

`gen.MUTATION_WEIGHTS` maps each stable mutation-op key to a relative pick
weight (`mutate`/`mutate_with_info` take an optional `weights` override in
the same format; every draw goes through the caller's `random.Random`, so a
given seed stays deterministic). Value mutations keep the document valid
while exploring values and are weighted 8x:

- `number` (boundary numbers), `insert` (`\relax`/`\par`/space),
  `wrap` (group/`\hbox`/`\vbox` wrap)

Structural mutations often break the input so both engines fail, and are
weighted 1x:

- `brace` (brace deletion/insertion), `catcode` (catcode change),
  `cs-swap-dup` (control-sequence swap/duplicate),
  `halign` (alignment `&`/`\cr` separator swap)

When a mutation returns text identical to its seed, the harness re-draws
(up to 3 times, without running any engine) before spending engine time.

## Dedupe (signatures)

Every non-`equal`, non-`both-fail` input gets a signature.
`capture()` returns the transcript log and the return code but not the
engine's stderr, so when an input crashes the harness re-runs that same
input directly with `subprocess` (same args and environment, its own
temp dir, the same timeout, stderr kept to its last 64 KiB) and builds
the crash signature from that stderr:

- panics: the panic site `panic:<file>:<line>`, parsed from
  `panicked at <file>:<line>:<col>` (column and thread id dropped), e.g.
  `panic:crates/foo/src/bar.rs:1033`. A new panic site is a new signature.
- signals: the signal name plus the first stderr line with every digit
  replaced by `N`, e.g. `signal:SIGABRT:fatal runtime error: stack
  overflow, aborting`. Different abort causes get different signatures.
- exit 101 with no panic text: `exit:101`.
- `diverge`: the first differing log line with every digit replaced by `N`.
- `both-crash`: `both-crash:` plus the candidate's crash signature above.
- `both-hang`: the constant `both-hang`.
- `timeout`: the constant `timeout`.

A case is stored under `OUT/<class>/<sha256-prefix>.tex` (with a `.json`
next to it: seed, mutation, return codes, first differing log line,
signature) only if its signature is new in this run and no case with that
signature exists on disk under `OUT`. Per-signature occurrence counts live
in `OUT/signatures.json` (merged across runs). A one-line summary prints
per 100 iterations plus a final count per class. Exit 0 always unless the
harness itself fails.

## Minimise

```sh
python3 tools/fuzz/minimize.py --candidate BIN --oracle BIN \
  --input case.tex --class diverge|candidate-crash|oracle-crash \
  --out min.tex --timeout SEC
```

Delta debugging (ddmin) by line first, then by token. A reduction is kept
only if it reproduces the SAME finding, not just its class: for `diverge`
in the default strict mode the raw `(candidate line, oracle line)` pair
at the first difference must be byte-identical to the original pair
(harness normalisation only, digits included), so a decoy divergence of
the same digit-masked shape cannot replace the real bug; `--loose` falls
back to the digit-masked comparison for minimising across changing
numbers. For crashes the stderr-based crash signature must be unchanged
in both modes, so the minimized case is the same bug. Classification and
signatures reuse `run.classify`/`run_one`/`crash_signature`/`crash_stderr`;
nothing is duplicated. Writes `min.tex` and prints which mode ran, the
position of the first difference for the original and the minimised case
(`box B line O`: shipped-box number and line offset inside that box's
dump, or the plain compared-log `line N` outside any box), a warning when
the position moved more than line/token deletion explains (warning only),
and the number of engine runs used (each candidate+oracle pair counts as
2, each crash-signature re-run as 1, plus one final verification pair).

Note: the lockstep capture normalises accounting only (memory usage, PDF
statistics, output byte count); everything else compares byte-exact.

## Tests

```sh
python3 -m unittest discover -s tools/fuzz
```

Fake shell-script engines only, in the style of
`tools/lockstep/test_capture.py`; the real candidate is never used.

## Parser fuzzers and the document generator

`tools/fuzz/parsers/{tfm,type1,png,jpeg,pdfinc}.py` fuzz the candidate's
file parsers (no oracle, only the no-panic contract) and `docgen.py` is a
document-level differential generator (see `parsers/README-*.md` and
`README-docgen.md`). All take `--candidate BIN` (required) and read
`FLASHTEX_POOL` / `FLASHTEX_FORMATS` from the environment; without them the
candidate cannot start and every run looks like a graceful error.
`python3 -m unittest discover -s tools/fuzz` does not descend into
`parsers/`; run `python3 -m unittest discover -s tools/fuzz/parsers`.

## Nightly

`nightly.py` runs every fuzzer (`run.py`, `docgen.py`, and
`parsers/{tfm,type1,png,jpeg,pdfinc}.py`) one after another as
subprocesses inside a wall-clock budget. Each fuzzer gets a base target
iteration count (see `FUZZERS` in `nightly.py`); the driver first runs
20 probe iterations of each to estimate seconds per iteration, then
scales every target by one factor so the estimated total stays inside
the budget, and never plans past the remaining time.

Wall-clock enforcement is hard: each fuzzer subprocess runs in its own
process group (`start_new_session=True`) with a timeout of its
budget share times 1.2 plus 30 seconds. On overrun the whole group gets
SIGTERM, then SIGKILL after 5 s; the fuzzer is recorded as `timed-out`
in `summary.json`/`summary.md`, no new fuzzer starts once the budget
plus 60 seconds has passed, and the exit code is 2 if any fuzzer had
to be killed.

```sh
FLASHTEX_POOL=$HOME/engine/pdftex.pool FLASHTEX_FORMATS=$HOME/engine/fmt \
python3 tools/fuzz/nightly.py --candidate BIN --oracle BIN --out DIR \
  --budget-minutes 60 --lockstep-cases tools/lockstep/cases
```

`--seed` defaults to the number of days since 1970-01-01 in UTC, so each
night differs and a night replays with the logged seed. `OUT/summary.json`
holds per-fuzzer iterations, class counts, new-signature counts, elapsed
seconds and seeds; `OUT/summary.md` holds one table plus, for every
finding, the artifact path and its sidecar json. Exit 0 means no findings,
only findings listed in `known-findings.json` (a trailing `*` is a prefix
match, e.g. `fontcount-diff:*`; an entry with a `fuzzers` list, e.g.
`["type1"]`, only matches findings from those fuzzers, entries without
it are global), or only known-benign `both-crash` /
`both-hang` / `both-flood` findings (pdfTeX's own crashes/hangs/floods:
listed in the summary but exit 0); exit 1 means a new finding; exit 2
means a harness failure (including a fuzzer killed for overrunning its
wall-clock timeout).
New findings are triaged as engine-diff issues.

## Findings so far

- **Type 1 self-recursive subroutine (both engines crash).** A charstring
  subroutine that calls itself (`callsubr` to its own index) overflows the
  stack: pdfTeX 1.40.29 dies with SIGSEGV (exit 139) and the candidate aborts
  with `fatal runtime error: stack overflow` (SIGABRT, exit 134). Found by
  `parsers/type1.py` (mutation `cs-recursion`, 12 of 2000 iterations, seed 202
  against a `cmr10.pfb` seed). pdfTeX's own crash, so this is listed apart from
  the engine-diff issues; DESIGN §4.5 (no panics) still wants the candidate to
  fail gracefully. The seed font is a TeX Live file and is not committed.
- **Header `\hbox{ }` with microtype `spacing`** (candidate panics, pdfTeX
  exits 0) is #1219; `docgen.py` rediscovers it in about 7% of documents,
  including the `\oddfoot` variant.
