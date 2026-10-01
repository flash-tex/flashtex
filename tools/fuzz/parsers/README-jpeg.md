# JPEG parser fuzzer (`tools/fuzz/parsers/jpeg.py`)

Crash fuzzer for JPEG image parsing (`\pdfximage`). Stdlib only.

## Seeds

Real JPEGs from the TeX Live tree, resolved at run time with
`kpsewhich`: `example-image.jpg`, `example-image-a/b/c.jpg` (all
baseline SOF0, from `tex/latex/mwe`), plus a progressive seed and
a 12-bit precision variant (see below). When no seed resolves,
a hand-built 1x1 baseline JPEG is generated (verified rc=0 on both
real pdfTeX and the candidate).

Job per iteration (in a temp dir as `fuzz.jpg` + `job.tex`):

```
\pdfximage{fuzz.jpg}\setbox0\hbox{\pdfrefximage\pdflastximage}
\shipout\box0 \bye
```

## Mutations (one per iteration, via `random.Random`)

Raw: byte flips, truncation at a random offset, JPEG segment deletion
or duplication. Marker-aware: SOF precision (now including 12) /
height / width / component-count edits (length/count fields take 0,
1, 0xFFFF, 0x7FFFFFFF, 0xFFFFFFFF, truncated to field width),
SOF0<->SOF2 kind flip, arbitrary segment-length rewrite, APP0 insert
with declared length 0xFFFF, truncate-before-EOI, SOF/DHT/DQT drop
or duplicate.

Progressive scans: in-place SOS spectral-selection rewrite
(`Ss/Se` in 0, 1, 63, 0xFF; `Ah/Al` nibbles in 0, 1, 13, 15) or an
extra SOS header with boundary spectral fields (two `FF DA`
headers). Restart markers: DRI insert/interval rewrite (0, 1, 8,
0xFFFF) or 1-4 RSTn (`FF D0`-`FF D7`) inserts. Metadata: EXIF APP1 /
ICC APP2 inserts with declared length `0, 1, 8, actual-1,
actual+10, 0xFFFF` (never equal to actual). Precision: SOF byte
forced to 12.

## Progressive and 12-bit seeds

`find_progressive_seed()` tries `kpsewhich
example-image-progressive.jpg`, then `ls` (`os.listdir`, never a
recursive find) of TeX Live's `tex/latex/mwe` and `doc/latex/mwe`,
keeping the first JPEG whose headers parse with SOF2. TeX Live 2026
ships no progressive JPEG (all `mwe` images are SOF0), so
`load_seeds()` normally appends a synthetic `SOF0->SOF2` variant
(with boundary `Ss/Se/Ah/Al` fields) plus a 12-bit variant of the
first baseline seed.

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
