# flashtex-typst-host

This is FlashTeX's Typst engine host, phase T0 of [DESIGN.md §15](../docs/design/engine-v2/DESIGN.md) (with the phased §15 from PR #1264).

It runs as a separate process, one per open Typst document. It speaks the engine-host socket protocol of [`display-list-v3`](../docs/protocol/display-list-v3.md) §6, the same one `flashtex-host` speaks for LaTeX, and streams display-list pages. A client that says `[3, 2]` in its `HELLO` also gets the draft 3.2 additions (§15.4), which are described in `src/v32.rs`.

- **Licence:** our code is MIT. It links the unmodified Apache-2.0 `typst` crates, pinned exactly at `=0.15.1`, plus the MIT `flashtex-display-list`. It never links or copies from `crates/flashtex-engine`: `scripts/check-license-boundary.sh`, check D, enforces that. Obligations are listed in [NOTICE](NOTICE) and [licenses/](licenses/).
- **Workspace:** it has its own workspace, `Cargo.lock` and target directory, and is never a member of the root workspace (§15.2). LaTeX builds, tests and perf baselines never see Typst's dependency tree.

```sh
cargo build --release --locked --manifest-path typst-host/Cargo.toml
cargo test --locked --manifest-path typst-host/Cargo.toml
python3 typst-host/licenses/third_party.py --check --write DIR   # licence texts + NOTICE files
target/release/flashtex-typst-host --socket /tmp/t.sock --font-path DIR [--no-system-fonts]
```

## What T0 does

- **`HELLO`:** major 3. The minor is negotiated as the lower of 2 and the client's minor. Any other major gets `ERROR version`, and anything other than `HELLO` first gets `ERROR protocol`.
- **`COMPILE`:** the keys are `root`, `main`, `output_dir`, `jobname`, `buffers`, `edits`, `have_fonts`, `font_formats`, `incremental` and `export`. The compile itself is the standard `typst::compile`. Then the host sends:
  - `DIAGNOSTIC`s, with file, line and a 0-based byte `column`, plus `hints`;
  - `FONT`, `SOURCES` and `PAGE` in page order, each resource before its first use;
  - `PAGES`, for incremental clients;
  - exactly one `DONE`.
- **`DONE.pdf`:** typst-pdf's export of the same document, with no timestamp. This is Typst's oracle.
- **Pages:** each carries the page fill, glyphs (OpenType glyph ids, E1), paths with fill and stroke (colour quantised to u8 as the PDF has it), clips, URI and page links, and spans and columns. With 3.2, pages also carry `ORIGINS_F64` (E2) and `PAGE_META` (E7). Constructs v3 cannot draw are flagged INCOMPLETE with an `UNSUPPORTED` entry: gradients and tilings, images, alpha, spot colour and stroked text. A 3.1 client gets every page with glyphs INCOMPLETE and falls back to `DONE.pdf`.
- **Incremental:** with `incremental: true`, a later compile keeps resource and span ids and sends only the pages whose Typst `hash128` changed.
- **`World`:** every path is confined to the canonical project root, so symlinks cannot escape it. `@preview` packages are refused and the network is never used. Fonts come from files only, because the host is built without `embedded-fonts`.
- **Writes (`buffers`, `edits`):** these never follow a symlink. The path is walked from the root with `openat`, every component opened `O_NOFOLLOW`, and the opened descriptor's own path is re-checked against the root before anything is truncated.
- **Fonts on the wire:** there is one FONT frame per font instance (program, face and variation coordinates). Each program is hashed, held and sent once per connection; later instances of it carry `program_from`, a draft 3.2 key. Ids are checked: past 65,536 instances the compile fails with an error instead of wrapping the u16 id.
- **Memory:** `comemo::evict(10)` runs after every compile's pages are out.

## Not in T0 (§15.10 T1 and later)

- PDF-derived f64 origins, the zero-pixel route of §15.5. That encoding is owned by P3-ZERO-TOLERANCE.
- The seeded 1-pass loop and its idle re-check.
- `viewport`-first ordering.
- The package lock, offline mode and consent.
- The watchdog, which is the app's job.
- `lang-v1`.
- v3.2 E3–E6 and E8.
- Moving `src/v32.rs` into `flashtex-display-list`. The protocol owner does that.

## Tests

- **`tests/protocol.rs`:** round trips against the running host, decoded by the MIT reference decoder. They cover:
  - negotiation and refusals;
  - message order and resources before use;
  - the 3.2 sections and hashes, and the 3.1 fallback;
  - held fonts and incremental compiles;
  - request errors, diagnostics, confinement and packages;
  - superseded compiles.
- **`tests/oracle.rs`:** four sample documents in `tests/fixtures/` go end to end through the host. Each one is checked against the pinned typst, compiled in-process by an independent `World`:
  - `DONE.pdf` is byte-identical to that compile's export;
  - pages, sizes and glyph ids are the same;
  - every glyph's f64 origin matches where the PDF's content stream puts it (`tests/pdfpos`, a small independent PDF text interpreter).
