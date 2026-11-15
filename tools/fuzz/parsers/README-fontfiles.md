# VF, map and encoding fuzzer (`tools/fuzz/parsers/fontfiles.py`)

A project can carry its own virtual fonts, font maps and encoding files, and
the engine reads them from the project folder before the TeX tree. This
fuzzer mutates one such file per iteration and runs the candidate on a job
that loads it (lane TRANSFER-AUDIT, sandbox finding 5). `--kind` picks the
file:

- `vf`: `fuzz.vf`, a mutated `ptmr8t.vf` or `pplr8t.vf` (or a synthetic VF),
  with the seed's own TFM as `fuzz.tfm`; job `\font\x=fuzz \x Az\bye`.
  VF-aware edits: a `fnt_def` renamed to `fuzz` (a virtual font naming
  itself), `fnt_def` fields set to edge values, char packet headers past
  the file, and DVI opcodes put into a packet (an undefined font, pushes
  without pops, pops without pushes, huge specials, rules and moves).
- `map`: `fuzz.map`, made from map-line pieces (quotes, `<`, `<<`, `<[`,
  `SlantFont`/`ExtendFont` with edge numbers, long and many lines, NUL and
  non-ASCII bytes) and mutated; job
  `\pdfmapfile{=fuzz.map}\font\x=fuzz \x a\bye`.
- `enc`: `fuzz.enc`, a mutated `8r.enc` (or a synthetic one): brackets or
  `def` missing, more than 256 names, very long names, bare names; job
  `\pdfmapline{=fuzz CMR10 "FuzzEncoding ReEncodeFont" <fuzz.enc <cmr10.pfb}`.

```sh
python3 tools/fuzz/parsers/fontfiles.py --kind vf --candidate BIN \
  --iterations N --seed S --out DIR --timeout SEC
```

The candidate runs as `BIN -fmt=pdftex -interaction=nonstopmode job.tex`
with `FLASHTEX_POOL`, `FLASHTEX_FORMATS` and `SOURCE_DATE_EPOCH=0`. Classes:
`ok`, `rejected` (a pdfTeX error or warning about the file), `graceful-error`,
`crash` (a signal or exit 101), `hang` (over `--timeout`), `output-flood`.
Crash, hang and flood inputs are stored as `OUT/<class>/<sha256-prefix>.<kind>`
with a `.json` sidecar, one per distinct signature. Seeds come from the TeX
tree through `kpsewhich` at run time. Every draw goes through one
`random.Random(seed)`.

`tools/fuzz/nightly.py` runs the three kinds every night (T6) and
`.github/workflows/font-fuzz.yml` runs them with the TFM, Type 1 and
TrueType fuzzers on demand. Tests: `python3 tools/fuzz/parsers/test_fontfiles.py`.
