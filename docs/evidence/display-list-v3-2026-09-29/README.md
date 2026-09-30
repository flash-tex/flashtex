# display-list-v3: the engine's preview output, exact to pdflatex's PDF (2026-09-29)

Lane **P3-DISPLAYLIST** (kabir-claude, mac-m5pro-kabir), DESIGN.md §3, §6.1,
§6.2, §12 P3. Branch `agent/kabir-claude/p3-displaylist`, from main with
#1198 (P2-PT1-FIXTURES: the command line) and #1202 (P3-IMAGES: image and
PDF inclusion) merged in, since neither had landed.

Specification: [docs/protocol/display-list-v3.md](../../protocol/display-list-v3.md) (MIT).

## What was built

| piece | where | licence |
|---|---|---|
| wire format: framing, PAGE/FORM bodies, FONT/IMAGE/SOURCES, content hash, SHA-256, JSON; blocking Unix-socket client; `dl3-dump`, `dl3-client` | `crates/display-list-v3` (`flashtex-display-list`) | MIT |
| display-list writer: stream capture, content-stream interpreter in exact decimal arithmetic, source side table, links/dests, font and image resources | `crates/flashtex-engine/src/displaylist/` | GPL-2.0-or-later |
| hooks in the traversal and node allocation (calls only) | `crates/flashtex-engine/changes/displaylist.ch` → `src/generated/` (regenerated) | GPL-2.0-or-later |
| engine host: Unix-socket server, one engine process per compile, frames relayed from fd 3, diagnostics, cancel | `crates/flashtex-engine/src/host/main.rs` (`flashtex-host`) | GPL-2.0-or-later |
| positions checker (independent PDF interpreter in `Fraction`s) | `tools/displaylist/check_positions.py` | tooling |
| socket round-trip benchmark | `tools/displaylist/bench_socket.sh` | tooling |

The engine depends on the MIT crate (one definition of the format for both
sides); nothing MIT depends on the engine. `scripts/check-license-boundary.sh`:
clean.

### How positions are obtained (and why they are exact)

The writer does not re-implement pdfTeX's traversal or any layout. pdfTeX's
`pdf_ship_out` calls the C routines `pdfshipoutbegin`/`pdfshipoutend`
around the page's content stream; the Rust ports of those (utils.rs) start
and finish a capture, and `write_pdf`/`write_zip` (output.rs) hand over the
bytes of each PDF-buffer flush in between. At `pdfshipoutend` the stream is
complete; `src/displaylist/interp.rs` reads it as a PDF viewer does: `Tf`,
`Td`/`TD`/`Tm`/`T*`, `TJ` adjustments, each glyph's advance from the
`/Widths` pdfTeX will write for the font (`divide_scaled(char_width,
pdf_font_size, 4)` tenths, computed at the same point), `cm`, `q`/`Q`,
colour, paths, clips, `Do`. All in fixed-point decimals of 10⁻¹² bp, which
hold every number pdfTeX prints exactly; a glyph origin is rounded once to
sp. Which node drew what comes from `dl_node(p)` marks the traversal leaves
in the stream (the offset at which it outputs node p), and where node p came
from from the side table `dl_new_node` fills at allocation.

The hooks change no variable of TeX's: with `FLASHTEX_DISPLAY_LIST` unset they
are one relaxed atomic load; set, they write only the writer's own tables.
`mem`, node sizes and the globals are pdftex.web's (no SyncTeX fields).

## Verified results (measured on mac-m5pro-kabir, 2026-09-29)

Engine: this branch's `flashtex-initex` (release), `pdflatex.fmt` built by it
from TeX Live 2026's `pdflatex.ini`. Oracle: TeX Live 2026 `pdftex` 1.40.29,
`SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, cached by tools/parity/tiers.py.
qpdf 12.4.2.

### Positions: 82/82 parity fixtures exact

`python3 tools/displaylist/check_positions.py --engine target/release/flashtex-initex --formats FMTDIR -j 6 --json positions-fixtures.json`

**82/82 documents exact (0 sp): 175 pages, 93,509 glyphs, 700 rules**, and
every form XObject the pages use (beamer-madrid, beamer-visuals). Per
document: `positions-fixtures.json` beside this file (pages, glyphs, rules,
forms, display-list bytes, unsupported operators). Two fixtures
(beamer-madrid, beamer-visuals) use `gs` (transparency) and are
flagged INCOMPLETE; their glyphs and rules still compare exact.

Each fixture is compiled to convergence by the engine with the display list
on; the display list of the converged pass is compared with **pdflatex's**
PDF, whose positions the checker computes on its own (a PDF content
interpreter in rational arithmetic over the qpdf-normalised streams, with
the fonts' `/Widths` from the PDF), in stream order, per page and per form
XObject. Glyphs compare (font, code, x, y) and rules (kind, left, top,
width, height), all at 0 sp.

Negative control: the same comparison with the checker's rounding changed
from nearest to floor finds 1,936 of hw1's 2,743 glyphs different, so a
difference of 1 sp is caught.

### Behaviour unchanged

- **P-T1 82/82 and P-T2 82/82 with the display list off** (the default) and
  **82/82 and 82/82 with it on** (`--engine-env FLASHTEX_DISPLAY_LIST=/dev/null`):
  `python3 tools/parity/parity.py --tier fixtures --engine target/release/flashtex-initex --engine-env FLASHTEX_FORMATS=FMTDIR --engine-env FLASHTEX_POOL=... [--engine-env FLASHTEX_DISPLAY_LIST=/dev/null] --pt on --raster none`.
  Writing the display list changes no box dump, no `\tracingall` log line
  and no PDF byte that P-T1/P-T2 compare.

- trip (`scripts/flashtex-trip.sh`) and etrip (`scripts/flashtex-etrip.sh`,
  whose scratch package now also builds `src/displaylist/` and depends on
  the MIT crate): pass.
- drift test (`cargo test -p web2rust`): pass; `src/generated/` is the
  translator's output of the change files, 19 lines of calls.

### Socket round trip

`tools/displaylist/bench_socket.sh FMTDIR`: the host runs the engine per
compile; the reference client measures from sending `COMPILE`. Warm =
after two compiles that converge the `.aux` files, with `have_fonts` (the
client already holds the font programs), medians of 4:

| fixture | pages | first compile bytes | warm bytes | first page (ms) | DONE (ms) | first page (ms), 2nd run | DONE (ms), 2nd run |
|---|---|---|---|---|---|---|---|
| plain-article | 3 | 598,677 | 86,827 | 141.3 | 159.1 | 220.5 | 245.2 |
| hyperref-toc | 3 | 2,548,658 | 116,418 | 207.2 | 244.1 | 380.1 | 443.4 |
| lecture-notes | 2 | 1,353,473 | 72,704 | 167.3 | 188.3 | 273.0 | 304.3 |
| thesis-chapter | 5 | 2,295,104 | 181,533 | 167.9 | 204.4 | 198.4 | 236.5 |
| beamer-madrid | 7 | 190,540 | 84,700 | 439.0 | 648.4 | 440.3 | 573.5 |

The machine was shared with other agents' builds (load average 18–26 during
the second run, `bench-socket-raw.jsonl`); the first run was earlier and
quieter. Either way the time to the first page is the engine's own time to
reach the first `\shipout` (format load and the preamble); the display
list adds a few ms and the socket well under 1 ms.

Transport alone (the host running a stand-in engine that replays the
display lists of all 82 fixtures, 47.8 MB of frames, onto fd 3): the host
relay, the socket and the client's decoding together sustain 2,146 (539 before both ends widened the Unix socket buffers from macOS's default 8 KiB to 4 MiB) MB/s.
Page decoding in the Rust client (`dl3-dump --bench`, the 82 fixtures'
211 PAGE/FORM frames, 1.62 MB): 1,285 MB/s, 82.5 million items/s (best of 20).

Engine overhead of writing the display list (hyperref-toc, 7 alternating
runs each): 230.6 → 246.5 ms median on a quieter machine (+16 ms: font programs read
and hashed, pages interpreted, frames written; the node hooks were +45 ms
before tokens scanned under `scanner_status` ≠ normal were skipped and the
side table moved out of thread-local storage, and SHA-256 used the ARMv8
instructions); 378.8 → 406.8 ms under load. `lecture-notes`: 169.2 →
173.5 ms.

### Tests

- `crates/flashtex-engine/tests/display_list_host.rs`: host + MIT client end
  to end: builds `pdflatex.fmt`, compiles `fixtures/real-world/hyperref-toc`
  three times, receives every page as it ships (fonts and spans always
  before the page that uses them, page count equal to the PDF's), named
  links resolve to destinations, >1000 glyphs carry source spans, then a
  cancelled compile. Skips without TeX Live.
- `crates/display-list-v3/tests/client_protocol.rs`: the client against a
  scripted host (message order, content hash, cancel, major-version
  refusal, an unknown opcode fails closed).
- Unit tests: exact text positions and rules from a hand-written stream,
  markers to spans, decimal arithmetic, encodings, `/FontMatrix` for
  slanted/extended fonts, the eqtb locations the writer reads
  (`\count0-9`, `\mag`) checked against the translation, SHA-256 known
  answers and the ARMv8 path against the software path, page encode/decode
  round trip.
- The spec's Swift sketch type-checks with `swiftc` 6.4 and decodes the 82
  fixtures' display lists to the same 175 pages, 102,151 items and 93,509
  glyphs as the Rust decoder.

## Remaining gaps, by owner

1. **INCOMPLETE pages** (2 of the 82 fixtures: beamer's transparency
   `gs`): extended graphics state, shadings, patterns and inline images are
   flagged, not expressed. Owner: this lane (next: ExtGState alpha from the
   page resources; shadings as a resource).
2. **Span granularity**: text produced by a macro takes the position where
   the macro call ended (SyncTeX does no better); running heads have none.
   Owner: this lane with P4 (the incremental system needs finer spans).
3. **Time to first page** is the engine's preamble time (format load and
   packages): P4's resident engine (L1) removes it; the host already streams
   pages as they ship.
4. **App integration** (Swift, behind a flag; Type 1 rendering, DESIGN §6.2):
   the mac-claude-a lane, from spec §8–§9.

## Reproduce

```sh
CARGO_BUILD_JOBS=4 cargo build --release -p flashtex-engine -p flashtex-display-list
# pdflatex.fmt with the engine, in FMTDIR:
(cd FMTDIR && FLASHTEX_POOL=$PWD/crates/flashtex-engine/pdftex.pool \
   target/release/flashtex-initex -ini -jobname=pdflatex -progname=pdflatex -translate-file=cp227.tcx '*pdflatex.ini' </dev/null)
python3 tools/displaylist/check_positions.py --engine target/release/flashtex-initex --formats FMTDIR -j 6
tools/displaylist/bench_socket.sh FMTDIR
cargo test --release -p flashtex-engine --test display_list_host
cargo test --release -p flashtex-display-list
```
