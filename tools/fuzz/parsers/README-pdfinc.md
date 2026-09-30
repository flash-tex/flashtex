# pdfinc: `\pdfximage` crash fuzzer

Fuzzes PDF files included as images. Seeds are 4 small PDFs compiled by
the reference pdfTeX from tiny documents at run time (one page, two
pages, halign box, font use); nothing third-party is copied into the
repo. Each iteration mutates one seed and runs the job
`\pdfximage{fuzz.pdf}\setbox0\hbox{\pdfrefximage\pdflastximage}
\shipout\box0 \bye` (every ~4th uses `\pdfximage page 2`) through the
candidate with `FLASHTEX_POOL`, `FLASHTEX_FORMATS`, `SOURCE_DATE_EPOCH=0`.

## Run

```sh
python3 tools/fuzz/parsers/pdfinc.py --out DIR --iterations 300 \
  --seed 1 --timeout 10 [--candidate ~/engine/bin/pdftex]
```

Classes: `crash` (signal, exit 101, or `panicked at`), `hang`
(timeout), `ok` (exit 0), `graceful-error` (other nonzero). Crash/hang
inputs are saved to `OUT/<class>/<sha256-prefix>.pdf` with a `.json`
sidecar (`seed`, `mutation`, `returncode`, last stderr line, panic
location), deduped by panic location or signal. Deterministic given
`--seed` (all choices via `random.Random`).

Mutations: byte flips, truncation at random offsets, chunk deletion /
duplication, plus structure-aware edits (xref offsets, `startxref`,
drop trailer, `/Length`, `/Count`, `/Kids` cycle, object reference
cycles, huge object-stream `/N`, truncate inside a stream). Length/count
fields are set to 0, 1, 0xFFFF, 0x7FFFFFFF or 0xFFFFFFFF.

## Tests

```sh
python3 tools/fuzz/parsers/test_pdfinc.py
```

Fake shell-script engines only (one panics when the input PDF contains a
marker string); the real candidate is never used.
