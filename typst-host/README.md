# flashtex-typst-host

This is FlashTeX's Typst engine host, phase T0 of [DESIGN.md §15](../docs/design/engine-v2/DESIGN.md) (with the phased §15 from PR #1264).

It runs as a separate process, one per open Typst document. It speaks the engine-host socket protocol of [`display-list-v3`](../docs/protocol/display-list-v3.md) §6, the same one `flashtex-host` speaks for LaTeX, and streams display-list pages. A client that says `[3, 3]` in its `HELLO` also gets the Typst additions of DESIGN.md §15.4, now version 3.3 of the shared specification ([§11](../docs/protocol/display-list-v3.md)) and of the `flashtex-display-list` crate.

- **Licence:** our code is MIT. It links the unmodified Apache-2.0 `typst` crates, pinned exactly at `=0.15.1`, plus the MIT `flashtex-display-list`. It never links or copies from `crates/flashtex-engine`: `scripts/check-license-boundary.sh`, check D, enforces that. Obligations are listed in [NOTICE](NOTICE) and [licenses/](licenses/).
- **Workspace:** it has its own workspace, `Cargo.lock` and target directory, and is never a member of the root workspace (§15.2). LaTeX builds, tests and perf baselines never see Typst's dependency tree.

```sh
cargo build --release --locked --manifest-path typst-host/Cargo.toml
cargo test --locked --manifest-path typst-host/Cargo.toml
python3 typst-host/licenses/third_party.py --check --write DIR   # licence texts + NOTICE files
target/release/flashtex-typst-host --socket /tmp/t.sock --font-path DIR [--no-system-fonts] \
    [--package-cache DIR] [--package-path DIR]... [--package-mirror URL] [--offline]
```

## What T0 does

- **`HELLO`:** major 3. The minor is negotiated as the lower of 3 and the client's minor. Any other major gets `ERROR version`, and anything other than `HELLO` first gets `ERROR protocol`.
- **`COMPILE`:** the keys are `root`, `main`, `output_dir`, `jobname`, `buffers`, `edits`, `have_fonts`, `font_formats`, `incremental` and `export`. The compile itself is the standard `typst::compile`. Then the host sends:
  - `DIAGNOSTIC`s, with file, line and a 0-based byte `column`, plus `hints`;
  - `FONT`, `SOURCES` and `PAGE` in page order, each resource before its first use;
  - `PAGES`, for incremental clients;
  - exactly one `DONE`.
- **`DONE.pdf`:** typst-pdf's export of the same document, with no timestamp. This is Typst's oracle.
- **Pages:** each carries the page fill, glyphs (OpenType glyph ids, E1), paths with fill and stroke (colour quantised to u8 as the PDF has it), clips, URI and page links, and spans and columns. With 3.3, pages also carry `ORIGINS_F64` (E2) and `PAGE_META` (E7). Constructs v3 cannot draw are flagged INCOMPLETE with an `UNSUPPORTED` entry: gradients and tilings, images, alpha, spot colour and stroked text. A 3.1 or 3.2 client gets every page with glyphs INCOMPLETE and falls back to `DONE.pdf`.
- **Incremental:** with `incremental: true`, a later compile keeps resource and span ids and sends only the pages whose Typst `hash128` changed.
- **`World`** (T1, `src/world.rs`): every path is confined to the canonical project root, and a package's files to the package's canonical root, so symlinks cannot escape them. Fonts come from files only, because the host is built without `embedded-fonts`.
- **Packages** (T1, `src/packages.rs`, spec §11.8): looked up vendored in the project (`typst-packages/<ns>/<name>/<ver>/`), then in `--package-path` directories, then in the cache (`--package-cache`, default the user's cache directory `FlashTeX/typst-packages`), then, for `@preview` only and only when the `COMPILE` says `"packages": "online"` (the app's consent), fetched from packages.typst.org (`--package-mirror`) by the system `curl` on a background thread. **Offline is the default**; `--offline` forbids fetching whatever a client says. The compile that starts a fetch waits at most 200 ms and any other none; then it fails with a located diagnostic, and a client that accepts `packages-v1` gets `PACKAGE` messages (`needed`, `fetching`, `ready`, `failed`) and compiles again. A failed fetch is retried after 5 s, doubling up to 5 minutes. `curl` runs by absolute path with `-q` (no `~/.curlrc`), capped at 64 MiB while reading.
- **The project's lock** (T1, `src/lock.rs`): `flashtex-typst.lock` in the project root, written by the host and meant to be committed. `[packages]` holds each package tarball's SHA-256, recorded on the first fetch and checked on every later fetch and every use of the cached copy; a mismatch is a located error and the package is not used. The unpacked tree in the cache is checked against its tarball once per process (same files, same bytes, nothing more) and replaced by a fresh unpack if it differs. `[fonts]` holds the SHA-256 of each font file the document's text uses, recorded on the first successful compile (`src/fontlist.rs`); a missing font, another variant, or another file is a `DIAGNOSTIC` with `"kind": "font"` on every compile until fixed or accepted (`COMPILE` `"lock": "update"`). Fonts are recorded and checked after `DONE`, never before the edited page, so a finding reaches the client with the next compile. The lock is written under an exclusive `flock` on the project root after re-reading it, to a new file renamed over it, never through a symlink; a later version's tables are kept verbatim, and a lock that cannot be read is reported and left alone.
- **Writes (`buffers`, `edits`):** these never follow a symlink. The path is walked from the root with `openat`, every component opened `O_NOFOLLOW`, and the opened descriptor's own path is re-checked against the root before anything is truncated. A file with more than one hard link is refused.
- **Fonts on the wire:** there is one FONT frame per font instance (program, face and variation coordinates). Each program is hashed and held once per connection.
  - A client whose HELLO `accept` lists `font-program-refs` (spec §11.1) gets each program once; later instances carry `program_from`.
  - Any other client gets the whole program in every FONT that takes one (spec §5.1), bounded by a per-compile budget (`--font-program-budget`, default 256 MiB). Past the budget the compile fails with a diagnostic.
  - Ids are checked: past 65,536 instances the compile fails with an error instead of wrapping the u16 id.
- **Memory:** `comemo::evict(10)` runs after every compile's pages are out.

## Not in T0 (§15.10 T1 and later)

- PDF-derived f64 origins, the zero-pixel route of §15.5. That encoding is owned by P3-ZERO-TOLERANCE.
- The seeded 1-pass loop and its idle re-check.
- `viewport`-first ordering.
- Vendoring the cache's packages into the project (an app action over the documented layout), and `@preview` index browsing.
- The watchdog, which is the app's job.
- `lang-v1`.
- v3.3 E3–E6 and E8.

## Tests

- **`tests/protocol.rs`:** round trips against the running host, decoded by the MIT reference decoder. They cover:
  - negotiation and refusals;
  - message order and resources before use;
  - the 3.3 sections and hashes, and the 3.1/3.2 fallback;
  - held fonts and incremental compiles;
  - request errors, diagnostics, confinement and packages;
- **`tests/world.rs`:** packages vendored, offline (with `needed`), fetched from a `file://` mirror in the background (`PACKAGE` events, the lock recorded), a changed tarball refused on fetch and from the cache, package files confined, the font list recorded, changed and missing fonts reported on every compile, `update` and `off`, and a symlinked or newer lock left alone. No test touches the network.
  - superseded compiles.
- **`tests/oracle.rs`:** four sample documents in `tests/fixtures/` go end to end through the host. Each one is checked against the pinned typst, compiled in-process by an independent `World`:
  - `DONE.pdf` is byte-identical to that compile's export;
  - pages, sizes and glyph ids are the same;
  - every glyph's f64 origin matches where the PDF's content stream puts it (`tests/pdfpos`, a small independent PDF text interpreter).
