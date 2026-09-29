# JPEG parser fuzzer (`tools/fuzz/parsers/jpeg.py`)

Crash fuzzer for JPEG image parsing (`\pdfximage`). Stdlib only.

## Seeds

Real JPEGs from the TeX Live tree, resolved at run time with
`kpsewhich`: `example-image.jpg`, `example-image-a/b/c.jpg` (all
baseline SOF0, from `tex/latex/mwe`). Progressive coverage comes from
the `sof-kind` mutation (SOF0 rewrites to SOF2). When no seed resolves,
a hand-built 1x1 baseline JPEG is generated (verified rc=0 on both
real pdfTeX and the candidate).

Job per iteration (in a temp dir as `fuzz.jpg` + `job.tex`):

```
\pdfximage{fuzz.jpg}\setbox0\hbox{\pdfrefximage\pdflastximage}
\shipout\box0 \bye
```

## Mutations (one per iteration, via `random.Random`)

Raw: byte flips, truncation at a random offset, JPEG segment deletion
or duplication. Marker-aware: SOF precision / height / width /
component-count edits (length/count fields take 0, 1, 0xFFFF,
0x7FFFFFFF, 0xFFFFFFFF, truncated to field width), SOF0<->SOF2 kind
flip, arbitrary segment-length rewrite, APP0 insert with declared
length 0xFFFF, truncate-before-EOI, SOF/DHT/DQT drop or duplicate.

## Run

```sh
python3 tools/fuzz/parsers/jpeg.py --candidate BIN --iterations N \
  --seed S --out DIR --timeout SEC
```

`--candidate` defaults to `$HOME/engine/bin/pdftex`. Each run is
deterministic for a given `--seed`. Classes: `crash` (signal, exit
101, or `panicked at`), `hang` (over `--timeout`), `ok` (exit 0),
`graceful-error` (other non-zero). Crash/hang inputs are saved once
per signature (panic location, else signal/exit/timeout) as
`OUT/<class>/<sha256-prefix>.jpg` plus a `.json` sidecar (`seed`,
`mutation`, `returncode`, `last_stderr_line`, `panic_location`).

## Tests

```sh
python3 tools/fuzz/parsers/test_jpeg.py
python3 -m unittest discover -s tools/fuzz
```

Fake shell-script candidates only (panic-on-marker, sleeper); the
real engine is never used.
