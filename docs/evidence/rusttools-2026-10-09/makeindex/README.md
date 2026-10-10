# makeindex in Rust, re-verified on current main (lane RUST-TOOLS, 2026-10-09)

Lane **RUST-TOOLS** (mac-claude-a, mac-m1max-a), Commander lanes of 2026-10-09 on #1319: no
external programs at user time; in bundle mode (no TeX Live) bibtex, biber and makeindex were
unavailable (NO-TEXLIVE audit, #1319, 2026-10-07, gap 4). MacTeX's `makeindex` is the oracle
only.

## What this branch is

The makeindex port of lane COLD-SPEED (`agent/mac-claude-a/cold-makeindex`, 2026-10-04, never
pushed): `crates/makeindex` (TeX Live 2026's makeindex 2.18, `texk/makeindexk` at
`6a300188053b8f2ded89dbd52293732a706b9c0e`, function by function), run in-process for restricted
`\write18` and for the host's latexmk makeindex rule, `.ist` files through the engine's kpathsea
(`Format::Ist`), so it needs no TeX Live binary; with a bundle the bundle's kpathsea finds the
styles. Design, licence placement and the first evidence: [the COLD-SPEED
README](../../cold-speed-2026-10-04/makeindex/README.md).

This branch merges current `origin/main` (`adc0ee899`) into it. Two conflicts, both additive:
`resolver.rs` (main added `Format::Pict` beside the branch's `Format::Ist`) and `system.rs`
(main added `with_resolver_for`; the port needs `with_resolver` visible to the crate).

## Identity with TeX Live 2026's makeindex, after the merge (VERIFIED)

Port binary: `flashtex-makeindex` built from this branch (release). Byte for byte on exit
status, standard output, standard error and every file left in the directory.

| run | cases | mismatches | raw |
|---|---|---|---|
| fuzzer (`../../cold-speed-2026-10-04/makeindex/harness/fuzz.py`, seeds 0-2999: random `.idx` with `@ ! \| "` and `\`, 1-4 levels, encaps, `\|(`/`\|)` ranges, see/seealso, every page-number type and composites, malformed lines, CR LF; random local styles, TeX Live styles, options `-q -c -l -r -g -s -o -t -p -i -L -T`, `LC_ALL` variants) | 3,000 (oracle exit 0: 2,579, exit 1: 389, never ends: 32) | **0** | `raw/fuzz-0-3000*` |
| corpus (`corpus.sh`): 18 `.idx` (the three multi-pass index fixtures, makeidx; the imakeidx book's `idx` and `notation`, imakeidx with `\makeindex[...]`; TeX Live's 13 makeindex test inputs: `tort`, `tortW`, `sample`, ranges, `pprecA/B`, letter ordering `romalpA-D`, `toodeep`) × 64 styles (none, all 54 TeX Live 2026 `.ist` by kpathsea, 9 local: `dots.ist` and TeX Live's `pprec0-7.ist`) × 4 option sets (none, `-q`, `-l` letter ordering, `-r -c`) | 4,608 | **0** | `raw/corpus*` |

The COLD-SPEED runs on the branch before the merge (10,000 fuzz cases, 6,160 corpus runs
including *Infinite Descent*'s four indexes, and engine `\write18` passes) also had 0
mismatches; the merge changed none of the port's files.

Machine: M1 Max, load1 290-490 during these runs (other sessions); no timing is claimed here.

## Licence (recorded)

makeindex is under the **MakeIndex Distribution Notice** (`texk/makeindexk/COPYING`):
permissive; a modified version (a port is one) must carry an identical notice and say where its
source is. `crates/makeindex/LICENSE` carries the notice verbatim; only the GPL engine links the
crate (`scripts/check-license-boundary.sh`, check F). A release that ships the engine must
include that notice. Belief, not verified: compatible with distributing the engine under GPL-2,
like the vendored libraries of DESIGN.md §3. **Open for the owner's §3 legal review before any
public release:** whether the MakeIndex Distribution Notice's conditions (identical notice,
source available) may be combined with GPL-2 distribution of the engine that links the port, and
the transcripts' program banners ("This is makeindex, version 2.18 [TeX Live 2026] ...",
printed because the output is byte-identical to TeX Live's).

## Hardening after review (2026-10-09)

The port reads an input file whole, where the C program streams it, so a document could make a
host that runs it in-process read without bound (`\write18{makeindex /dev/zero}`, a `.idx` that
links to `/dev/zero`: gigabytes in seconds, then an out-of-memory abort). Now:

- **Inputs** (`.idx`, `.ist`, `.log`) must be regular files after following links and at most
  `MAX_INPUT` (64 MiB); a FIFO, a device or a larger file is refused with
  `makeindex: refusing to read NAME: not a regular file` (or the size) on standard error, and
  makeindex then fails as for a missing file. Standard input is read up to the same limit. A
  directory still opens and reads as empty, as with `fopen`. This differs from TeX Live only
  where TeX Live's program would hang or exhaust memory. Tests: `crates/makeindex/tests/limits.rs`
  (a link to `/dev/zero`, as input and as style; a FIFO; a 64 MiB + 1 sparse file) and
  `tests/write18.rs` (`makeindex_refuses_dev_zero`).
- The host's rule reads and hashes its inputs (`.aux`, `.bib`, `.idx`) under the same rule
  (`host/external.rs`, `read`): a `refs.bib` linked to `/dev/zero` reads as missing.
- **Panics:** `flashtex_makeindex::run` catches a panic in the port (reported as
  `KILLED_BY_SIGSEGV`), and the host's tools thread catches any panic of a rule's run, so the
  engine thread always gets `ToolsDone` and `doc.tools.running` is cleared; the rules' memory
  lock no longer propagates a poisoned lock.
- **Confined reads** (`FLASHTEX_CONFINE_READS`, Live Share): the port's reads go through the
  engine's confinement (`system::tool_read_ok`: the name rule, and the resolved file in an
  allowed root or a link-free TeX tree file found by kpathsea), for `\write18` and the host rule.
- **Windows** (`\write18` command splitting, `makeindex::command_args`): cmd.exe has no `'`
  quotes and expands `%VAR%` even inside `"..."` (texmfmp.c quotes restricted arguments with
  `"` and leaves `%` alone), and the C runtime treats `\"` specially; such commands now go to the
  shell, which does all that, instead of running in-process.

## Reproduce

```sh
cargo build --release -p flashtex-engine --bin flashtex-makeindex
P=$PWD/target/release/flashtex-makeindex
python3 docs/evidence/cold-speed-2026-10-04/makeindex/harness/fuzz.py --port $P --seeds 0:3000 --jobs 4 --out fuzz.jsonl
sh docs/evidence/rusttools-2026-10-09/makeindex/corpus.sh $P
```
