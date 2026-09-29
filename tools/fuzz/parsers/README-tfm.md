# TFM fuzzer (`tools/fuzz/parsers/tfm.py`)

Mutates real `.tfm` seeds (resolved at run time with
`kpsewhich cmr10.tfm cmmi10.tfm cmsy10.tfm ptmr8t.tfm`; nothing copied
into the repo) and runs each as `fuzz.tfm` next to a job file holding
`\font\x=fuzz \x a \bye` through the candidate as
`BIN -fmt=pdftex -interaction=nonstopmode job.tex` with
`FLASHTEX_POOL`, `FLASHTEX_FORMATS` and `SOURCE_DATE_EPOCH=0`.

```sh
python3 tools/fuzz/parsers/tfm.py --candidate BIN --iterations N \
  --seed S --out DIR --timeout SEC
```

Classes: `ok` (exit 0), `graceful-error` (nonzero, no crash),
`crash` (signal, exit 101, or `panicked at`), `hang` (over `--timeout`).
Mutations: byte flips, truncation at a random offset, chunk
deletion/duplication, and header-field edits (one of the 12 u16 header
words set to `0`, `1`, `0xFFFF`, `0x7FFFFFFF` or `0xFFFFFFFF`).
Crash/hang inputs are stored as `OUT/<class>/<sha256-prefix>.tfm` with a
`.json` sidecar (`seed`, `origin`, `mutation`, `returncode`, `last_line`,
`panic_location`, `signature`), deduped by panic location or signal, so
one stored case per distinct bug. All draws go through one
`random.Random(seed)`: the same seed reproduces the same run.

Tests (`python3 tools/fuzz/parsers/test_tfm.py`): fake shell-script
candidate that panics when `fuzz.tfm` contains a marker byte string;
checks determinism, class counts, and crash/hang artifact writing.
