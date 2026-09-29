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
- `candidate-crash`: candidate died by signal, exited 101, or printed
  `panicked at`.
- `oracle-crash`: same, for the oracle.
- `both-fail`: both engines non-zero without crashing (not interesting).
- `timeout`: either engine exceeded `--timeout`.

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

Every non-`equal`, non-`both-fail` input gets a signature:

- `candidate-crash` / `oracle-crash`: the panic location (text after
  `panicked at` up to the first colon-number pair, e.g.
  `src/main.rs:123`) or, without one, the signal / exit code.
- `diverge`: the first differing log line with every digit replaced by `N`.
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
  --input case.tex --class diverge|candidate-crash --out min.tex \
  --timeout SEC
```

Delta debugging (ddmin) by line first, then by token. A reduction is kept
only if it reproduces the SAME class; for `candidate-crash` the panic
location must also match, so the minimized crash is the same bug.
Classification reuses `run.classify`/`run_one`; nothing is duplicated.
Writes `min.tex` and prints the number of engine runs used (each
candidate+oracle pair counts as 2).

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
