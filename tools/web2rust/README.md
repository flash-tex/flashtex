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
cargo run --release -p web2rust -- third_party/knuth/tex.web \
    @crates/flashtex-engine/web2rust-default.args \
    --out-dir crates/flashtex-engine/src/generated \
    --pool crates/flashtex-engine/tex.pool
```

`@FILE` reads further arguments from FILE (whitespace-separated, `#` comments).
The engine's configuration lives in two such files, so the regeneration
command, the drift test and the trip build cannot disagree:

- `crates/flashtex-engine/web2rust-default.args` — the committed build: TeX
  Live 2026's `texmf.cnf` capacities for `tex` wherever `tex.web` can hold them
  (the file explains each value and the exceptions).
- `crates/flashtex-engine/web2rust-trip.args` — tripman.tex step 2's
  capacities, used only by `scripts/flashtex-trip.sh`.

## Drift check

```sh
cargo test --release -p web2rust --test drift
```

regenerates into a temporary directory with the default configuration and
fails if any file of `crates/flashtex-engine/src/generated/` or `tex.pool`
differs. A translator change therefore lands together with its regenerated
output. Never edit `src/generated/` by hand.

## Standing in for a WEB change file

`tex.web` is never edited, so what web2c gets from `tex.ch` and `texmf.cnf`
are options here:

| option | overrides |
|---|---|
| `--const NAME=VALUE` | an outer-block Pascal constant (`mem_max`, `error_line`, ...) |
| `--macro NAME=VALUE` | a WEB macro whose body is a number (`mem_bot`, `mem_top`, `max_halfword`, `hash_size`) |
| `--scalar NAME=f32` | narrows a named `real` type |
| `--stat`, `--debug` | make `stat`/`tats` and `debug`/`gubed` empty |

`--stat` compiles the usage-statistics code in, as web2c does and as the trip
test expects. `--scalar glue_ratio=f32`: `tex.web` §109 declares
`glue_ratio=real` and marks it a system dependency; web2c narrows it to a C
`float`, and so did the run that produced Knuth's master `trip.log`. With `f64`
eight `\vbox` `glue set` values and three DVI movements in the trip test differ
in the last digits; with `f32` `trip.log` is byte-identical.

Code changes (as opposed to values) cannot be expressed yet; the first that
needs one is lifting `font_max <= 256` (§111 case 16), which comes with
pdftex.web and a change-file mechanism.

## Trip test

```sh
scripts/flashtex-trip.sh
```

generates the trip configuration into a scratch package (never into
`src/generated/`), builds it without kpathsea, runs tripman.tex steps 3, 4 and
6 against the committed fixtures in `third_party/knuth/trip/`, and exits
non-zero on any failure. CI runs it on Ubuntu and macOS (job `trip`). Result on
2026-09-29:

| output | result |
|---|---|
| `tripin.log` | byte-identical (465 lines) |
| `trip.log` | byte-identical (7306 lines; sha256 `61a65352…` as Knuth's master) |
| `tripos.tex` | byte-identical |
| `tripin.fot`, `trip.fot` | identical after exactly two accepted differences: the typed lines Knuth's terminal echoed, and the final newline `tex.web` §1333 does not print (`tools/web2rust/tools/trip_fot.py`) |
| `trip.typ` (DVItype, where installed) | identical except DVItype's own banner line |

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
