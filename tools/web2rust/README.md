# web2rust

A translator from the WEB Pascal subset to Rust, per
[`docs/design/engine-v2/DESIGN.md`](../../docs/design/engine-v2/DESIGN.md) §4.1
(decision D3, step 1). It is MIT-licensed, like the rest of the tooling; the
crate it *generates* is GPL-2.0-or-later because its input is.

It does two jobs:

1. **Tangle** (`src/tangle.rs`) — the TANGLE-equivalent front end: WEB module
   expansion (`@<...@>`), numeric / simple / parametric macros (`@d`), the
   `@'` octal and `@"` hex constants, `@$` pool checksum, `@&` joins,
   `@{ ... @}` meta-comments, control text (`@t @^ @. @:`), and the string pool
   in `tex.pool` format.
2. **Translate** (`src/parse.rs`, `src/emit.rs`) — parse the Pascal subset and
   emit Rust that keeps every identifier and carries the WEB section number as
   `// §NNNN`.

## Regenerating the engine crate

From the repository root:

```sh
cargo run --release -p web2rust -- \
    third_party/knuth/tex.web \
    --stat --scalar glue_ratio=f32 \
    --out-dir crates/flashtex-engine/src/generated \
    --pool crates/flashtex-engine/tex.pool
```

`--stat` defines WEB's `stat`/`tats` as empty rather than `@{`/`@}`, i.e.
compiles the usage-statistics code in, which is what web2c does and what the
trip test's `trip.log` expects. Omit it to reproduce what a stock TANGLE
produces.

`--scalar glue_ratio=f32` makes that one `real` type 32 bits wide. `tex.web`
S109 declares `glue_ratio=real` and marks it a system dependency; web2c narrows
it to a C `float`, and so did the run that produced Knuth's master `trip.log`.
With `f64` the trip test's `glue set` values differ in the last digits on eight
`\vbox` lines, and three DVI `down4`/`y4` movements differ; with `f32`
`trip.log` is byte-identical. It also makes `memory_word` exactly 32 bits,
which is what S113 describes.

## Standing in for a WEB change file

`tex.web` is never edited, so the two things web2c gets from `tex.ch` and
`texmf.cnf` are options here:

| option | overrides |
|---|---|
| `--const NAME=VALUE` | an outer-block Pascal constant (`mem_max`, `error_line`, ...) |
| `--macro NAME=VALUE` | a WEB macro whose body is a number (`mem_bot`, `mem_top`) |
| `--scalar NAME=f32` | narrows a named `real` type |
| `--stat`, `--debug` | make `stat`/`tats` and `debug`/`gubed` empty |

## Trip test

`scripts/flashtex-trip.sh <fixture-dir>` regenerates with the capacities
tripman.tex step 2 requires, builds, runs steps 3, 4 and 6, and diffs against
Knuth's masters. Result on 2026-09-29:

| output | differing lines |
|---|---|
| `tripin.log` | 0 of 465 |
| `trip.log` | **0 of 7306** (sha256 identical to Knuth's master) |
| `tripos.tex` | 0 of 3 |
| `trip.typ` (DVItype on `trip.dvi`) | 1 of 1214, DVItype's own version banner |
| `trip.fot` | 2, both terminal-transcript artefacts (see the script) |

Note that the script rewrites `crates/flashtex-engine/src/generated/`; restore
it with the regeneration command above.

## Oracle check against Knuth's TANGLE

MacTeX's `tangle` is an oracle only — it is never in the product path. The check
is that the token stream and the string pool agree exactly:

```sh
scripts/web2rust-oracle-check.sh
```

which runs `tangle tex.web` in a scratch directory, tokenises the resulting
`tex.p` with `tools/web2rust/tools/pascal_tokens.py`, and diffs it against
`web2rust --emit-pascal`. Verified result on 2026-09-29 with `tex.web` version
3.141592653: **120,560 tokens identical, `tex.pool` byte-identical including the
`*504454778` checksum.**

Identifiers are deliberately *not* normalised the way TANGLE normalises them
(TANGLE upper-cases them and deletes underscores); the comparison lower-cases
and strips underscores on both sides. Everything else — including TANGLE's
constant folding in `send_val`/`send_sign`, which turns `mem_top-1` into
`29999` — is reproduced exactly.
