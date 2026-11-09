# JBIG2 image fuzzer (`tools/fuzz/parsers/jbig2.py`)

Fuzzes JBIG2 files included with `\pdfximage` (pdfTeX's writejbig2.c,
the engine's `src/pdftex/writejbig2.rs`), which no other fuzzer reached.
pdfTeX reads only the file header, the segment headers and the page
information segments of a JBIG2 file and copies segment data unread, so
the files are built from a segment model (ITU-T T.88 Annex D) and every
iteration changes one of those structures:

- segment number (which sets the width of the referred-to segment
  numbers: 1, 2 or 4 bytes), type (valid and reserved), page association
  (0, other pages, the 4-byte form), data length (0, off by one, `0xFFFFFFFF`
  "unknown");
- referred-to segments: to an existing, a missing, a later or the same
  segment, more than four (the long form), the long form forced;
- segment order: delete, duplicate, swap;
- the file header: sequential or random-access organisation, the
  "number of pages unknown" bit, other flag bits, the page count;
- page information: width, height, resolutions, flags, striping (an
  unknown height with end-of-stripe rows), a segment cut short;

and about one input in six also gets byte flips or a truncation. Seeds
(written by the fuzzer, nothing copied into the repo): a sequential
one-page file; a random-access file of two pages that both refer to a
page-0 symbol dictionary (`/JBIG2Globals`); a striped page of unknown
height. The job includes page 1, pages 1 and 2 (which share the page-0
segments), another page or page 0, with `\pdfminorversion=5` (sometimes
3, below what JBIG2 needs).

## Run

```sh
python3 tools/fuzz/parsers/jbig2.py --candidate BIN [--oracle pdftex] \
  --iterations 300 --seed 1 --out /tmp/jbig2-out --timeout 10
python3 -m unittest discover -s tools/fuzz/parsers
```

The candidate reads `FLASHTEX_POOL` / `FLASHTEX_FORMATS` from the
environment (never passed to the oracle); both engines run with
`SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1` and `-fmt=pdftex`.

## Classes

Without `--oracle` (the no-panic contract, as `png.py`): `crash` (signal
or exit 101), `hang` (over `--timeout`), `ok`, `graceful-error`,
`output-flood`.

With `--oracle` (differential; nightly runs it this way): the exit
status, the PDF bytes and the log from the `**` line on (lines joined,
the program name after `pdfTeX warning:` / `!pdfTeX error:` replaced, the
end-of-run accounting cut) must agree. Classes as in `run.py`: `equal`,
`diverge`, `candidate-crash`, `oracle-crash` (a signal; exit 101 from
pdfTeX is not a crash), `both-crash`, `timeout` (one side; the signature
says which), `both-hang`, `output-flood`, `both-flood`.

Every class but `ok`/`graceful-error`/`equal` is stored once per
signature as `OUT/<class>/<sha256-prefix>.jbig2` with a `.json` sidecar
(seed, mutation, the job, both return codes, the last output line, the
panic location). Deterministic given `--seed`.
