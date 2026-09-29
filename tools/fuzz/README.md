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

Every non-`equal`, non-`both-fail` input is stored under
`OUT/<class>/<sha256-prefix>.tex` with a `.json` next to it (seed,
mutation, return codes, first differing log line). A one-line summary
prints per 100 iterations plus a final count per class. Exit 0 always
unless the harness itself fails.

Note: the lockstep capture normalises accounting only (memory usage, PDF
statistics, output byte count); everything else compares byte-exact.

## Tests

```sh
python3 -m unittest discover -s tools/fuzz
```

Fake shell-script engines only, in the style of
`tools/lockstep/test_capture.py`; the real candidate is never used.
