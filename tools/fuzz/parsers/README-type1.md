# Type 1 font program fuzzer (`parsers/type1.py`)

Crash fuzzer for Type 1 font programs (`.pfb`), Python 3 stdlib only.

## Seeds and job

Seeds are read from the TeX Live tree at run time via `kpsewhich`:
`cmr10.pfb`, `cmti10.pfb`, `cmbx10.pfb` (plus a synthetic minimal PFB if
none is found). Each iteration copies `cmr10.tfm` as `fuzz.tfm`, writes the
mutated program as `fuzz.pfb`, and runs the candidate job:

```tex
\pdfmapline{+fuzz fuzz <fuzz.pfb}\font\x=fuzz \x abc \bye
```

Baseline verified 2026-09-29: the unmutated copy compiles and embeds the
font (`<./fuzz.pfb>` in the output) on both the reference
`/Library/TeX/texbin/pdftex` and the candidate
`$HOME/engine/bin/pdftex`, so the map line needs no adjustment.

## Mutations

Byte flips, truncation at random offsets, chunk deletion/duplication, and
format-aware edits: PFB segment-header type-byte rewrites, segment length
fields set to 0, 1, 0xFFFF, 0x7FFFFFFF, 0xFFFFFFFF, and eexec (segment
type 2) truncation. Deterministic given `--seed` (`random.Random`).

## CharStrings-level mutations

The eexec block is decrypted (Type 1 key `r=55665`), every
`/name len RD <len bytes> ND` CharStrings entry and
`dup idx len RD <len bytes> NP` Subrs entry is located (the declared
length must line up with the `ND`/`NP` terminator), one entry's body is
decrypted (per-charstring key `r=4330`, `lenIV=4` prefix preserved),
its code bytes are mutated, and the entry is re-encrypted (`r=4330`),
spliced back with its `RD` length updated, and the whole eexec block is
re-encrypted (`r=55665`) with the PFB segment lengths rebuilt — so the
file stays structurally valid while the charstring bytes are hostile:

- `cs-operand`: one charstring number becomes a boundary value (the
  1/2/5-byte encoding ranges: 0, 1, ±107/±108, ±1131/±1132, 255/256,
  ±32767/±32768, int32 extremes).
- `cs-operator`: one operator byte sequence is replaced with (or
  inserted as) `hsbw`, `rlineto`, `rrcurveto`, `callsubr`,
  `callothersubr`, `div`, `seac`, `closepath` or `endchar`.
- `cs-subr`: the `callsubr`/`callothersubr` argument becomes huge or
  negative (100000, 32767, 1000000, -1, -2, -1000, int32 extremes), or
  such a call is inserted when the charstring has none.
- `cs-recursion`: a Subrs entry's body becomes `<own index> callsubr`,
  i.e. unbounded self-recursion when the subr runs.
- `cs-endchar`: the trailing `endchar` is dropped (or the last byte
  when there is no `endchar`).

Unit tests cover exact decrypt/encrypt round-trips (synthetic vectors
plus the real `cmr10.pfb` eexec block when `kpsewhich` finds it) and
assert every `cs-*` mutation keeps the container valid (segments tile,
`RD` lengths match, eexec re-decrypts).

## Run

```sh
python3 tools/fuzz/parsers/type1.py --candidate $HOME/engine/bin/pdftex \
  --iterations 300 --seed 1 --out /tmp/t1out --timeout 20
```

The candidate inherits the harness environment plus `SOURCE_DATE_EPOCH=0`;
`FLASHTEX_POOL`/`FLASHTEX_FORMATS` default to `$HOME/engine/pdftex.pool`
and `$HOME/engine/fmt` when unset.

Classes: `crash` (signal, exit 101, or `panicked at`), `hang`
(timeout), `ok`, `graceful-error`. Every crash/hang input lands in
`OUT/<class>/<sha256-prefix>.pfb` with a `.json` sidecar (seed,
mutation, returncode, last stderr line, panic location); repeat bugs
dedupe by panic location or signal in `signatures.json`.

## Tests

Fake shell-script candidates only (panic-on-marker, always-ok,
always-fail, sleeper); the real candidate is never used:

```sh
python3 -m unittest tools.fuzz.parsers.test_type1
python3 -m unittest discover -s tools/fuzz
```
