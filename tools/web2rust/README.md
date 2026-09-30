# web2rust

A translator from the WEB Pascal subset to Rust (Knuth's `tex.web` and TeX
Live's `pdftex.web`), per
[`docs/design/engine-v2/DESIGN.md`](../../docs/design/engine-v2/DESIGN.md) §4.1
(decision D3, step 1). It is MIT-licensed, like the rest of the tooling; the
crate it *generates* is GPL-2.0-or-later because its input is.

It does three jobs:

0. **Change files** (`src/changes.rs`) — `@x`/`@y`/`@z` applied to the master
   exactly as TANGLE applies them (`--change FILE`, repeatable: several are
   applied in turn, as `tie -c` combines them). `--emit-web FILE` writes the
   merged WEB text.
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
cargo run --release -p web2rust -- third_party/pdftex/pdftex.web \
    @crates/flashtex-engine/web2rust-default.args \
    --out-dir crates/flashtex-engine/src/generated \
    --pool crates/flashtex-engine/pdftex.pool
```

`@FILE` reads further arguments from FILE (whitespace-separated, `#` comments).
The engine's configurations live in such files, so the regeneration command,
the drift test and the test builds cannot disagree:

- `crates/flashtex-engine/web2rust-default.args` — the committed engine:
  `pdftex.web` with the change files of `crates/flashtex-engine/changes/` and
  TeX Live's public-domain ones in `third_party/pdftex/web2c/`, and
  TeX Live 2026's `texmf.cnf` capacities for `pdflatex` (the file explains each
  value and the exceptions).
- `crates/flashtex-engine/web2rust-etrip.args` — the e-trip test's capacities
  (TeX Live's `etrip/texmf.cnf`), used only by `scripts/flashtex-etrip.sh`.
- `crates/flashtex-engine/web2rust-trip.args` — Knuth's `tex.web` with
  tripman.tex step 2's capacities, used only by `scripts/flashtex-trip.sh`
  (the P1 engine, kept as the reference for the TeX82 core).

## Drift check

```sh
cargo test --release -p web2rust --test drift
```

regenerates into a temporary directory with the default configuration and
fails if any file of `crates/flashtex-engine/src/generated/` or `pdftex.pool`
differs. A translator change therefore lands together with its regenerated
output. Never edit `src/generated/` by hand.

## Options that stand in for change files

What web2c gets from `texmf.cnf` is a value, not code, so it is an option:

| option | overrides |
|---|---|
| `--const NAME=VALUE` | an outer-block Pascal constant (`mem_max`, `error_line`, ...) |
| `--macro NAME=VALUE` | a WEB macro whose body is a number (`mem_bot`, `mem_top`, `max_halfword`, `hash_size`) |
| `--scalar NAME=f32\|i64` | narrows a named `real` type to 32 bits, or widens a named integer type to 64 |
| `--stat`, `--debug` | make `stat`/`tats` and `debug`/`gubed` empty |
| `--arena-cap NAME=EXPR` | the largest index of a growable (`^T`) array global, which reserves its region of the word space (see below) |
| `--index-type PATH` | wraps every array subscript in `PATH(...)`; the engine passes `crate::ix::U`, whose `Index` impls read without a bounds check in optimised builds and write through the checked barrier (`crates/flashtex-engine/src/ix.rs`) |

Code changes are change files (`crates/flashtex-engine/changes/`).

`--stat` compiles the usage-statistics code in, as web2c does and as the trip
tests expect.

**`glue_ratio`.** `tex.web` §109 declares `glue_ratio=real` and marks it a
system dependency. The two trip tests disagree about its width, and each is
followed:

- Knuth's master `trip.log` was made with a 32-bit real: with
  `--scalar glue_ratio=f32` it is byte-identical; with 64 bits eight `\vbox`
  `glue set` values and three DVI movements differ in the last digits. So the
  tex.web trip build uses `f32`.
- TeX Live's pdfTeX (and every web2c TeX) uses a C `double`
  (`texmfmp.h`, `GLUERATIO_TYPE`), which is why TeX Live's own trip filter
  lists those eight values as accepted differences. The e-trip test against
  TeX Live's pdfTeX (`scripts/flashtex-etrip.sh`) proves it: with `f64`
  every log and DVI file agrees with pdfTeX's; with `f32` (measured
  2026-09-29) 8 `glue set` lines of each trip log, 1 of `etripin.log` and 15
  of `etrip.log` (e.g. `1635.40002` for pdfTeX's `1635.4`) differ, and so
  does `trip.dvi`. So the engine, whose reference is pdfTeX, keeps
  pdftex.web's `real`.

## The word space

Every global array of plain-old-data elements is emitted as
`crate::arena::Arr<T>`, a region of one flat, zero-initialised allocation
(`crates/flashtex-engine/src/arena.rs`, DESIGN.md §4.2 and §5.2): reads are
plain loads, and every write goes through `IndexMut`, the checkpoint write
barrier. `Globals::new` lays the regions out in declaration order after a
region for the scalar globals, which `Globals::visit_scalars` lists in the
same order (the checkpoint copies them in and out); `Globals::visit_files`
lists the file globals. A web2c pointer array (`^T`, `xmalloc_array`) gets
its capacity from its single constant-size allocation, or, when it is grown
with `xrealloc_array`, from `--arena-cap` (the engine passes pdfTeX's
`sup_*` limits). An array type alias used as an element (`char_used_array`)
becomes a fixed-size Rust array.

## What pdftex.web needs beyond tex.web

pdftex.web is written for web2c, whose Pascal is C in disguise. The
translator follows web2c's reading wherever that differs from Pascal's, since
the C program is the one TeX Live ships:

- **Pointers.** `^T` is a C array allocated by `xmalloc_array(T, n)`
  (elements `0..n`) and grown by `xrealloc_array`; here it is a `Vec<T>`.
- **`external` routines.** pdfTeX's C parts are declared `external` in a
  change file; only their signatures are translated, and the bodies are
  hand-written Rust (`crates/flashtex-engine/src/pdftex/`).
- **`var` parameters** of any type, several per routine (e-TeX's `reverse`).
- **`return` without `exit:`.** web2c turns every `goto 10` into a C
  `return` (`web2c-parser.y`, `doreturn`); pdfTeX's own routines rely on it.
- **C conversions.** Booleans and integers mix (`is_bit_set:=(n div m) mod
  2`, `if byname>0`), as do characters and integers, and a real assigned to
  an integer is truncated; `longinteger` is 64-bit.
- **Precedence.** web2c prints `and`/`or`/`not` as `&&`/`||`/`!` without
  parentheses, and C binds `&&`/`||` more loosely than comparisons. Where
  that changes the meaning, web2rust refuses the expression (`parse.rs`,
  `precedence_clash`) and a change file adds the parentheses. `tex.web` has
  no such expression; `pdftex.web` has four.
- **Unary minus on a real** would be truncated by web2c's `- (integer)`; the
  emitter refuses it (none occurs except on literals, which web2c reads as
  negative constants).

The tangle stage needed nothing new: `web2rust --pool --emit-pascal` on
`pdftex.web` agrees with TANGLE 4.6 (TeX Live 2026) exactly, 230,400 tokens
and a byte-identical pool (1795 strings, checksum `400476366`), checked
2026-09-29 with `tools/pascal_tokens.py` as for tex.web below.

## Change files against TANGLE

`tests/changefile/` holds a small WEB program and a change file that exercise
comments, blank lines after `@x`, upper-case control codes, macro and
constant changes, a deletion, a multi-line match and a replacement that adds
a section, together with what TANGLE 4.6 writes for them
(`regenerate.sh`, which needs `tangle`). `cargo test -p web2rust --test
changefile` requires the same token stream and a byte-identical pool, and
that the two change files TANGLE rejects are rejected.

## e-trip test

```sh
scripts/flashtex-etrip.sh
```

builds the pdftex.web engine with `web2rust-etrip.args` in a scratch package,
runs the three parts of e-TeX's trip test, and compares every file with what
TeX Live 2026's pdftex writes (`third_party/pdftex/etrip-oracle/`), after the
normalisations the script lists. CI runs it on Ubuntu and macOS (job
`etrip`).

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
