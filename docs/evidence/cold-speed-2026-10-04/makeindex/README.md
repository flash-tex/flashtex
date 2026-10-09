# makeindex in-process (lane COLD-SPEED, item 2; 2026-10-04)

Lane **COLD-SPEED** (mac-claude-a, mac-m1max-a), Commander ruling 2026-10-04, item 2: port TeX
Live 2026's makeindex to Rust so the engine runs it in-process, with no TeX Live binary (the
owner's independence goal, DESIGN.md §4.4 D12), and so a document that calls makeindex through
restricted `\write18` (DESIGN.md §4.5) runs ours. MacTeX's `makeindex` is the oracle only.

**Machine.** M1 Max, shared: **load1 250-490 during this lane** (other sessions). No wall time
here is a reference; instructions retired (`/usr/bin/time -l`) are the measure.

## What was built

| piece | where |
|---|---|
| the port: `mkind.c`, `scanid.c`, `scanst.c`, `sortid.c`, `genind.c`, `qsort.c` (+ headers), function by function, statements in the C order | `crates/makeindex` (package `flashtex-makeindex`) |
| `\write18`: a command `shell_cmd_is_allowed` passed whose program is `makeindex` runs in-process (after `record_effect`, so the external effect and the L3 barrier are recorded exactly as before); arguments are the words `/bin/sh -c` makes of the re-quoted command; status reported as `system returned with code N` like the child's | `crates/flashtex-engine/src/system.rs` (`runsystem`, `in_process_tool`), `src/makeindex.rs` |
| the host's external tools (latexmk's `makeindex -o X.ind X.idx` rule) run it in-process in the scratch directory, terminal captured; it no longer needs a TeX Live `makeindex` | `src/host/external.rs` (`Runner::InProcess`) |
| `.ist` lookup through the engine's kpathsea: `Format::Ist` (`kpse_ist_format`, `INDEXSTYLE`), `kpse_out_name_ok` refusals reported as `makeindex: Not writing to ...` (new `flashtex_kpse_name_ok_silent`) | `src/resolver.rs`, `kpathsea-config/flashtex_kpse.c` |
| A/B switch | `FLASHTEX_MAKEINDEX=external` spawns TeX Live's program as before |
| the port as a command, exactly the `\write18` code path | `flashtex-makeindex` (engine bin) |
| tests: 8 golden cases with the oracle's output committed (no TeX Live needed), `\write18` in-process with **no makeindex on PATH** and the external fallback, the argument splitting | `crates/makeindex/tests/golden.rs`, `crates/flashtex-engine/tests/write18.rs`, `src/makeindex.rs` |

**Source of record.** <https://github.com/TeX-Live/texlive-source>, tag `texlive-2026.1`,
commit `6a300188053b8f2ded89dbd52293732a706b9c0e` (the pin of `third_party/pdftex`),
`texk/makeindexk`, makeindex **2.18** (ChangeLog 2026-01-12). The C sources were fetched to a
temporary directory for reference and are not committed (tools/parity/README.md).

**Reuse (DESIGN §1).** Evaluated: linking TeX Live's C makeindex in-process (as kpathsea is) and
existing alternatives (xindy, upmendex, texindy; no Rust makeindex exists). The C program is not
reusable in-process unmodified: it `exit()`s on errors, keeps all state in statics (one run per
process), writes `stdout`/`stderr` directly and changes the process locale with `setlocale`,
which is not thread-safe in the multi-threaded host. Making it re-entrant is a rewrite of every
function anyway; the alternatives are other programs with other output. So: a port.

## Licence placement (DESIGN §3)

makeindex is under the **MakeIndex Distribution Notice** (`texk/makeindexk/COPYING`): permissive,
but a modified version (a port is one, explicitly) must be covered by an identical notice, and
an executable without source must carry the notice and say where the source is. So the port is
**neither MIT nor GPL**: it is its own crate, `crates/makeindex`, whose `LICENSE` carries the
notice verbatim with the pin and where its source is. Only the GPL engine links it, as TeX
Live's own makeindex links LGPL kpathsea; `scripts/check-license-boundary.sh` gains **check F**
(no crate but the engine has `flashtex-makeindex` in its resolve graph; passes). Belief, not
verified: the notice's conditions (notice preserved, source available) are compatible with
distributing the engine under GPL-2; this is the same kind of question as the vendored
xpdf/zlib/libpng in §3, and it belongs in the owner's §3 legal review. Release notes of a
binary that links it must carry the notice (its own condition).

## Identity with TeX Live's makeindex (VERIFIED)

Byte for byte on **exit status, standard output, standard error and every file left in the
directory** (`.ind`, `.ilg`, anything else), each run in a fresh directory
(`harness/mkicmp.py`); the oracle runs as restricted `\write18` runs it (argv[0] `makeindex`
along PATH). Port binary: `flashtex-makeindex` built from this branch's commit.

| run | cases | mismatches | raw |
|---|---|---|---|
| **fuzzer** (`harness/fuzz.py`, seeds 0-9999): random `.idx` (keys with `@ ! \| "` and `\`, 1-4 levels, encaps, `\|(` `\|)` ranges, see/seealso, page numbers of every type and composite, 11-field pages, malformed lines, NUL bytes, CR and CR LF line ends, up to 1,600 entries), random local styles, TeX Live styles, options `-q -c -l -r -g -s -o -t -p -i -L -T`, refused output names, a second `.idx`, `LC_ALL` variants | **10,000** (1,276,838 entry lines; oracle exit 0: 8,613, exit 1: 1,270, never ends: 117) | **0** | `raw/fuzz-0-10000*.{jsonl,txt}` (coverage counts in `-coverage.txt`) |
| **corpus** (`harness/corpus.py`): 22 real `.idx` × 70 styles (none, all **54 TeX Live 2026 `.ist`** by kpathsea, 15 local: `dots.ist` and TeX Live's makeindex test styles) × 4 option sets (none, `-q`, `-l`, `-r -c`) | **6,160** | **0** | `raw/corpus*.{jsonl,txt}` |
| **engine `\write18`** (`harness/write18.py`): one engine pass per mode on fresh copies; every file left (157 for *Infinite Descent*, 11 for the imakeidx book) and the terminal output, in-process vs `FLASHTEX_MAKEINDEX=external`; the `.ind`/`.ilg` vs TeX Live's pdflatex | infdesc 2 × 2 passes (+ 2 × 2 before a rustfmt-only commit), idxbook 2 × 2 | **0** files differ; 8 + 4 `.ind`/`.ilg` identical to pdflatex's | `raw/write18-engine-passes*.jsonl` |
| golden (in `cargo test`, no TeX Live needed) | 8 | 0 | `crates/makeindex/tests/golden/` |

The 22 corpus `.idx`: the three `fixtures/multipass/*index*` documents and the imakeidx book
(`docs/evidence/p6-hyperopt-2026-10-04/raw/idx.tex`: `idx.idx`, `notation.idx`), each from one
TeX Live pdflatex pass on a temporary copy; *Infinite Descent*'s four (`infdesc`, `vocabulary`,
`notation`, `latex`: 494 + 34 + 94 + 111 entries, 9 of them rejected by makeindex, from one
pdflatex pass on a copy of `~/Documents/infdesc`, the original untouched); and TeX Live's own
makeindex test inputs (`tests/*.idx` at the pin, fetched to a temporary directory, not
committed: `tort`, `tortW`, `sample`, `nested-range`, `nested-range-bb`, `range`, `pprecA/B`,
`romalpA-D`, `toodeep`).

Before the final runs the fuzzer found, in this order: the `type_guess[10]`/`idx_keyword`
aliasing (53 of the first 200 cases), `scan_string`'s stale stack buffer, the non-terminating
`find_pageno`, and kpathsea's program name in refusals (an artefact of running the oracle by its
full path; fixed in the harness). Each is handled below; no mismatch remains.

### Behaviour the C code leaves undefined, and what the port does

TeX Live's binary is the reference, and where its undefined behaviour is deterministic and
visible the port reproduces it (each found by the fuzzer):

- **`type_guess[10]`**: `scan_no` indexes its static `int type_guess[10]` by the field count,
  which is 10 for a page number of 11 fields. In TeX Live 2026's macOS binary that element is
  the first 4 bytes of `idx_keyword`, so such a page number overwrites the keyword with a
  little-endian `int` (ARAB = 2 makes it `"\x02"`) and **every later `\indexentry` is an
  "Unknown index keyword"**. Verified on the oracle (a following `\x02{c}{2}` line is accepted);
  the port aliases that element the same way (golden case `eleven-fields`).
- `char` is signed (Apple arm64, x86): a delimiter byte >= 0x80 set by a style never equals a
  character read with `getc`; `0xFF` equals `EOF`.
- the static `key` buffer keeps earlier entries' bytes, which a quote before an embedded NUL
  reads; the shared `mk_getc` lookahead crosses streams; `scan_string`'s stack buffer keeps
  earlier strings, which "No closing delimiter in %s" prints.
- **find_pageno never ends** on a `.log` ending in CR LF (`-p odd/even/any`): reading the CR
  also reads the LF and the seek back returns to the CR. The oracle loops forever (the harness
  times it out and compares what it left); the port proves the repeated state and stops with
  `LOOPS_FOREVER` (the command exits 125; `\write18` reports -1, where pdflatex would hang).
- **Not reproducible, so not generated by the fuzzer and documented here:** a style file that
  ends inside a string *after an `fscanf` attribute* (the message prints stack bytes `fscanf`
  left: the one fuzz mismatch of the first run, seed 335, before the generator stopped making
  them); an empty `page_precedence` (an uninitialised array index); `page_compositor ""` (an
  out-of-bounds recursion); delimiters set to NUL. A crash path exists in genind.c (a NULL `prev`
  when the first sorted entry is a duplicate); it never occurred in 2,000 targeted oracle runs
  nor in the fuzzing, and the port reports `KILLED_BY_SIGSEGV` there instead of crashing.

**Locale.** makeindex sorts in the C locale but `new_entry` switches `LC_CTYPE` to the
environment's for group letters and headings, and `-L`/`-T` use `strcoll` in the environment's
`LC_COLLATE`. The port asks the C library for exactly that locale (`newlocale(mask, "")` and the
`_l` functions), so macOS's Latin-1 classification of bytes >= 0x80 in UTF-8 locales comes out
the same; `-L` and `-T` are implemented, and the fuzzer covers them and `LC_ALL` = `C`,
`en_US.ISO8859-1`, `th_TH.UTF-8` besides the session's `en_US.UTF-8`. On Windows (no `newlocale`)
the C locale is used.

## Measurements

All VERIFIED with `/usr/bin/time -l`, load1 350-490. Note what it counts: **instructions and
cycles of the timed process only** (checked: `sh -c "true; makeindex x.idx; true"` reports 24 M
instructions, `makeindex x.idx` alone 508 M), while **user+sys time includes waited-for
children** (wait4's rusage).

**makeindex alone** (`raw/makeindex-alone*`, median of 9, each in a fresh directory):

| `.idx` | TeX Live `makeindex` | via `/bin/sh -c` (what `\write18` spawns) | port (`flashtex-makeindex`) |
|---|---|---|---|
| one entry (baseline) | 509.9 M instr | 530.2 M | 523.6 M |
| infdesc (494 entries) | 525.0 M | 545.3 M | 533.9 M |
| latex (111) | 518.3 M | 539.0 M | 528.6 M |
| notation (94) | 514.5 M | 534.0 M | 527.0 M |
| vocabulary (34) | 509.9 M | 531.4 M | 524.2 M |

About 500 M of every run, for both programs, is kpathsea reading `texmf.cnf` (the first
`kpse_out_name_ok`; `kpsewhich -var-value=openout_any` alone is 507 M). The indexing itself,
above the one-entry baseline, is 19 M instructions for the four indexes in the port against 28 M
in TeX Live's binary. In-process the engine's kpathsea is already initialised, so the port pays
only the indexing.

**Per engine pass on *Infinite Descent*** (one pass of the 592-page book from a TeX Live
pass's `.aux`; four makeindex runs at `\printindex` through restricted `\write18`;
`raw/write18-engine-passes.jsonl`, 2 passes per mode, interleaved):

| | engine process | children | total |
|---|---|---|---|
| `FLASHTEX_MAKEINDEX=external` | 286.78 G, 286.85 G instr | 4 × `sh -c makeindex`: 2.15 G (sum of the medians above; 1.44 G cycles) | ≈ 288.97 G |
| in-process (default) | 286.78 G, 286.82 G instr | none | ≈ 286.80 G |

So a pass saves ≈ **2.15 G instructions (≈ 0.75 %)** and 1.44 G cycles, plus 4 process starts;
the in-process indexing is inside the engine's run-to-run spread (the two earlier passes with
the pre-rustfmt build: external 287.08/286.84 G, in-process 286.70/286.65 G). user+sys of the
pass (children included): 33.2/34.1 s external against 33.0/34.1 s in-process, inside the load
noise. On the 24-page imakeidx book (2 indexes) user+sys halves, 0.72/0.68 s against 0.39/0.40 s.
Belief, not measured unloaded: about 0.4-0.5 s of CPU per *Infinite Descent* pass at nominal
clock (1.44 G cycles), which a cold compile with several passes saves each pass. TeX Live's
pdflatex on the same pass: 292.5 G instructions.

## Not done / open

- The port is checked against the macOS arm64 TeX Live binary. On Linux, glibc's locale tables,
  unsigned `char` on arm64 and another static layout (the `type_guess[10]` aliasing) may make
  TeX Live's own binary behave differently in those undefined corners; not measured.
- `HELLO.texmf.tools.makeindex` still names TeX Live's program (or null); with the in-process
  runner makeindex is available either way. Not changed, to keep protocol 3.2 as documented.
- DESIGN §5.5 still says the makeindex port "comes later"; the Commander may want the text
  updated.

## Reproduce

```sh
cargo build --release -p flashtex-engine --bin flashtex-makeindex --bin flashtex-initex
P=target/release/flashtex-makeindex
H=docs/evidence/cold-speed-2026-10-04/makeindex/harness
python3 $H/fuzz.py --port $P --seeds 0:10000 --jobs 10 --out fuzz.jsonl
python3 $H/corpus.py --port $P --idx NAME=FILE.idx ... --local-ist STYLE.ist ... --jobs 10
python3 $H/measure.py --port $P --reps 7 FILE.idx ...
python3 $H/write18.py --engine-dir ENG --fmt-dir FMT --reps 2 NAME=SRC_DIR:ENTRY.tex
python3 $H/golden.py .   # rewrites crates/makeindex/tests/golden from the oracle
```

`raw/` holds the results (`*.jsonl`) and summaries of the runs quoted above.
