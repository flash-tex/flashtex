# TFM fuzzer (`tools/fuzz/parsers/tfm.py`)

Mutates real `.tfm` seeds (resolved at run time with
`kpsewhich cmr10.tfm cmmi10.tfm cmsy10.tfm cmex10.tfm ptmr8t.tfm`;
nothing copied into the repo) and runs each as `fuzz.tfm` next to a job
file holding
`\pdfmapline{+fuzz fuzz <cmr10.pfb}\font\x=fuzz \x a\bye`
(plus a copy of `cmr10.pfb`, also resolved with `kpsewhich`) through
the candidate as
`BIN -fmt=pdftex -interaction=nonstopmode job.tex` with
`FLASHTEX_POOL`, `FLASHTEX_FORMATS` and `SOURCE_DATE_EPOCH=0`.
The `\pdfmapline` gives the fuzz font a Type 1 map entry, so an
unmutated `cmr10` copy compiles and embeds cleanly: a font that loads
(`ok`) is distinguishable from one the engine rejects.

```sh
python3 tools/fuzz/parsers/tfm.py --candidate BIN --iterations N \
  --seed S --out DIR --timeout SEC
```

Classes: `ok` (exit 0), `tfm-rejected` (the log contains `Bad metric`
or `not loadable`, i.e. pdfTeX's `! Font \x=fuzz not loadable: Bad
metric (TFM) file.`), `graceful-error` (any other nonzero exit without
a crash), `crash` (signal, exit 101, or `panicked at`), `hang` (over
`--timeout`).
Mutations: byte flips, truncation at a random offset, chunk
deletion/duplication, header-field edits (one of the 12 u16 header
words set to `0`, `1`, `0xFFFF`, `0x7FFFFFFF` or `0xFFFFFFFF`), and
TFM-structure-aware edits on top of a header parse (`lf`, `lh`, `bc`,
`ec`, `nw`, `nh`, `nd`, `ni`, `nl`, `nk`, `ne`, `np` with per-table
byte spans): single-byte edits of individual `char_info` words,
width/height/depth/italic index bytes set out of range, `lig_kern`
program instruction edits (skip byte incl. `0`/`128`/`255` loop
candidates, next char, op byte, remainder incl. out-of-range labels
past `nl` and kern indices past `nk`), `exten` recipe byte edits, and
table-length edits that leave the declared tables inconsistent with
`lf` (no `exten` seed exists without `cmex10.tfm`; no `lig_kern` case
without a nonzero `nl` — those ops opt out and the generic ones
apply). `cmex10.tfm` (28 `exten` recipes, no `lig_kern` program) is in
the seed list for exactly this reason.
Crash/hang inputs are stored as `OUT/<class>/<sha256-prefix>.tfm` with a
`.json` sidecar (`seed`, `origin`, `mutation`, `returncode`, `last_line`,
`panic_location`, `signature`), deduped by panic location or signal, so
one stored case per distinct bug. All draws go through one
`random.Random(seed)`: the same seed reproduces the same run.

Tests (`python3 tools/fuzz/parsers/test_tfm.py`): fake shell-script
candidates (a marker panic, an always-ok exit, a `Bad metric` reject);
checks determinism, class counts (including `tfm-rejected`), the
`\pdfmapline` job, header parsing on a tiny self-consistent TFM, each
structure-aware mutation and its opt-out on empty tables,
`lf`-inconsistency of table-length edits, and crash/hang artifact
writing.
