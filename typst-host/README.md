# flashtex-typst-host

This is FlashTeX's Typst engine host, phases T0 and T1 of [DESIGN.md §15](../docs/design/engine-v2/DESIGN.md) (with the phased §15 from PR #1264).

It runs as a separate process, one per open Typst document. It speaks the engine-host socket protocol of [`display-list-v3`](../docs/protocol/display-list-v3.md) §6, the same one `flashtex-host` speaks for LaTeX, and streams display-list pages. A client that says `[3, 3]` in its `HELLO` also gets the Typst additions of DESIGN.md §15.4, now version 3.3 of the shared specification ([§11](../docs/protocol/display-list-v3.md)) and of the `flashtex-display-list` crate.

- **Licence:** our code is MIT. It links the unmodified Apache-2.0 `typst` crates, pinned exactly at `=0.15.1`, plus the MIT `flashtex-display-list`. It never links or copies from `crates/flashtex-engine`: `scripts/check-license-boundary.sh`, check D, enforces that. Obligations are listed in [NOTICE](NOTICE) and [licenses/](licenses/).
- **Workspace:** it has its own workspace, `Cargo.lock` and target directory, and is never a member of the root workspace (§15.2). LaTeX builds, tests and perf baselines never see Typst's dependency tree.

```sh
cargo build --release --locked --manifest-path typst-host/Cargo.toml
cargo test --locked --manifest-path typst-host/Cargo.toml
python3 typst-host/licenses/third_party.py --check --write DIR   # licence texts + NOTICE files
target/release/flashtex-typst-host --socket /tmp/t.sock --font-path DIR [--no-system-fonts]
```

## What it does

- **`HELLO`:** major 3. The minor is negotiated as the lower of 3 and the client's minor. Any other major gets `ERROR version`, and anything other than `HELLO` first gets `ERROR protocol`.
- **`COMPILE`:** the keys are `root`, `main`, `output_dir`, `jobname`, `buffers`, `edits`, `have_fonts`, `font_formats`, `incremental` and `export`. The compile itself is the standard `typst::compile`. Then the host sends:
  - `DIAGNOSTIC`s, with file, line and a 0-based byte `column`, plus `hints`;
  - `FONT`, `SOURCES` and `PAGE` in page order, each resource before its first use;
  - `PAGES`, for incremental clients;
  - exactly one `DONE`.
- **`DONE.pdf`:** typst-pdf's export of the same document, with no timestamp. This is Typst's oracle.
- **Pages:** each carries the page fill, glyphs (OpenType glyph ids, E1), paths with fill and stroke (colour quantised to u8 as the PDF has it), clips, URI and page links, and spans and columns. With 3.3, pages also carry `ORIGINS` (E2) and `PAGE_META` (E7). Constructs v3 cannot draw are flagged INCOMPLETE with an `UNSUPPORTED` entry: gradients and tilings, images, alpha, spot colour and stroked text. A 3.1 or 3.2 client gets every page with glyphs INCOMPLETE and falls back to `DONE.pdf`.
- **Positions (T1, DESIGN.md §15.5):** for a client that draws the pages (3.3 with `font_formats` `opentype`), every glyph's origin and glyph matrix, and the page box, are the ones the reference viewer computes from typst-pdf's export of that page (spec §4.2, §11.2), not Typst's frame positions (krilla writes f32, up to 6·10⁻⁵ bp off). The host exports the pages it sends with typst-pdf (untagged; the first page alone, so that it reaches the socket first, the rest in one export), reads each content stream with a small PDF reader (`src/pdf.rs`: cross-reference table, Flate) and interprets the text operators in the viewer's binary64 arithmetic (`src/pdfpos.rs`). Paths and clips are likewise the PDF's own (CTM, segments, line state: `PagePos::paths`), so a rule is drawn with the numbers the PDF draws it with (the 408-pixel case of §15.5). The walk consumes the PDF's glyphs run by run, including those of runs it does not draw (a gradient fill) and skipping an SVG image's inline text; when a run is not where the PDF shows it, or the export cannot be read, the page is INCOMPLETE rather than approximated. `DONE.positions_ms` says what it cost.
- **Seeded compiles (T1, DESIGN.md §15.3; spec §11.9):** an incremental compile runs Typst's layout loop seeded with the previous compile's introspector (`src/seeded.rs`, a re-implementation of 0.15.1's private `compile_impl` from public crates: re-verify it on every Typst release), usually one iteration instead of about four. Anything unusual (no previous document, any error, no convergence) falls back to the standard `typst::compile`; an `export` always uses it. The host re-checks a seeded compile against the standard one when idle (default after 1 s; `--verify idle:MS`), and when the pages differ sends them in a follow-up compile with `"cause": "verify"`; `--verify every` checks before sending, `--verify off` never; `--seeded off` disables the loop. `DONE` says `seeded`, `iterations`, `verified`.
- **Watchdog (T1, DESIGN.md §15.2; spec §11.10):** a thread watches each compile; past its wall-time budget (`--watchdog-secs`, default 10 s; `--watchdog-cold-secs`, default 180 s for a cold compile and for the idle check) or the RSS ceiling (`--rss-ceiling-mb`, default 4096) the host prints why and exits with status 86. The client restarts it and compiles cold. Tested on a hanging WASM plugin, a runaway `for` and runaway memory (`tests/watchdog.rs`).
- **Incremental:** with `incremental: true`, a later compile keeps resource and span ids and sends only the pages whose Typst `hash128` changed.
- **`World`:** every path is confined to the canonical project root, so symlinks cannot escape it. `@preview` packages are refused and the network is never used. Fonts come from files only, because the host is built without `embedded-fonts`.
- **Writes (`buffers`, `edits`):** these never follow a symlink. The path is walked from the root with `openat`, every component opened `O_NOFOLLOW`, and the opened descriptor's own path is re-checked against the root before anything is truncated. A file with more than one hard link is refused.
- **Fonts on the wire:** there is one FONT frame per font instance (program, face and variation coordinates). Each program is hashed and held once per connection.
  - A client whose HELLO `accept` lists `font-program-refs` (spec §11.1) gets each program once; later instances carry `program_from`.
  - Any other client gets the whole program in every FONT that takes one (spec §5.1), bounded by a per-compile budget (`--font-program-budget`, default 256 MiB). Past the budget the compile fails with a diagnostic.
  - Ids are checked: past 65,536 instances the compile fails with an error instead of wrapping the u16 id.
- **Memory:** `comemo::evict(3)` runs after every compile's pages are out (`--evict AGE`; DESIGN.md §15.2 said 10 until the 2026-10-04 measurement in `docs/evidence/typst-t1-2026-10-04/`: age 3 keeps 300 pages at 0.87 GB, flat).

## Not yet (§15.10 T1 and later)

- Colours from the PDF's numbers (they are still Typst's u8 components over 255, which the PDF writes as f32 decimals; measured pixel-exact on text in T0, not proven in general).
- `viewport`-first ordering.
- The package lock, offline mode and consent.
- `lang-v1`.
- v3.3 E3–E6 and E8.

## Tests

- **`tests/protocol.rs`:** round trips against the running host, decoded by the MIT reference decoder. They cover:
  - negotiation and refusals;
  - message order and resources before use;
  - the 3.3 sections and hashes, and the 3.1/3.2 fallback;
  - held fonts and incremental compiles;
  - request errors, diagnostics, confinement and packages;
  - superseded compiles.
- **`tests/oracle.rs`:** five sample documents in `tests/fixtures/` (text, math, shapes, links, transformed text with bleed) go end to end through the host. Each one is checked against the pinned typst, compiled in-process by an independent `World`:
  - `DONE.pdf` is byte-identical to that compile's export;
  - pages and glyph ids are the same;
  - **the positions checker** (T1's correctness gate): every glyph's origin and glyph matrix, every page box and every GLYPH's sp position are bit-identical to what `tests/checker` computes from that `DONE.pdf` (the whole tagged, compressed export; its own object scan, inflate and number reading, written independently of `src/pdf.rs`/`src/pdfpos.rs`). A test also shows the checker rejects Typst's frame positions.
- **`examples/positions_suite.rs`:** the same gate over Typst's own test suite: every snippet of `tests/suite` (Typst 0.15.1, with typst-dev-assets 0.15.1) that compiles goes through the host's conversion and is compared with the checker. It needs the two checkouts, so it is not a CI test:

  ```sh
  cargo run --release --locked --manifest-path typst-host/Cargo.toml --example positions_suite -- \
      --typst TYPST --assets TYPST_DEV_ASSETS --fonts TYPST_ASSETS/files/fonts --work /tmp/suite
  ```
