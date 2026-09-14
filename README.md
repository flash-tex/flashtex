# FlashTeX

FlashTeX is a blazing ⚡️ fast (La)TeX engine written in Rust. 
This project actually consists of two things:

- A _complete rewrite_ of the TeX compiler from scratch
  with modern features (incremental compilation, fancy diagnostics, etc.)
  
- A native, lightweight, and snappy TeX IDE with live (sub-10ms)
  previews, which pairs with a companion iPad app (FlashTeXPad)
  for inline LaTeX/TiKZ OCR (including diagrams).

## See Also

- Landing page: https://flash-tex.github.io/flashtex
- Quickstart/setup: https://flash-tex.github.io/flashtex/download
- Discord server: https://discord.gg/J4kHDJmTrD

---

**An incremental LaTeX engine with a command line, and a native macOS IDE
built on it — no TeX distribution required.**

FlashTeX is a Rust LaTeX engine. `flashtex build main.tex` lexes, lays out
and paints a document in tens of milliseconds, using the same TeX font
metrics as pdfLaTeX and the real Latin Modern faces (Computer Modern design;
New Computer Modern Math for the blackboard-bold and `amssymb` glyphs), and
writes the PDF with its own bundled writer — embedded font subsets, images,
links. The IDE, a Swift app for macOS, is a complementary project that links
nothing and drives the very same engine over a documented JSON Lines protocol
(`flashtex worker`): you type LaTeX and the page re-renders as you type. An
iPad companion (FlashTeXPad) turns Pencil sketches and photos into reviewed
LaTeX/TikZ insertions. Because the engine, the editor and the companions are
separate processes with versioned contracts, each is independently
replaceable and scriptable.

What it is **not** (yet): a full TeX engine. FlashTeX implements a growing
subset of LaTeX (article-class text, sectioning, lists, `amsmath`-style math,
`\newcommand`, `\input`/`\include`, labels and references, `tabular`,
`thebibliography`, TikZ basics); constructs it does not implement are
reported as diagnostics with a recovery note rather than silently dropped,
and most packages are still missing. `flashtex supported` prints the exact
inventory and coverage; see [Supported LaTeX](docs/user/compiler.md#supported-latex).

![FlashTeX rendering a real homework document: source on the left, live Latin Modern preview on the right, the Problems panel below](docs/images/flashtex-hw1.png)

## Features

- **The `flashtex` CLI** — `build` (multi-file projects, exact-route PDF),
  `check --json` (stable diagnostics report), `watch`, `supported`, `fonts`,
  and `worker` (the protocol the IDE speaks); `file:line:col` diagnostics,
  build-tool exit codes, fonts and metrics found next to the binary, so a
  tarball works with no TeX installation and no environment.
- **Workspace and multi-file projects** — a `.tex` file and its folder are the
  project; `\input`/`\include` targets appear in the sidebar with an outline of
  sections, environments and labels.
- **Editor** — LaTeX syntax highlighting, completion for commands, environments
  and labels (⌃Space), a command palette (⌘⇧P).
- **Per-keystroke incremental compile** — every edit is compiled by the bundled
  engine; only the paragraphs you touched are re-typeset.
- **Live preview** — an exact display list painted with Latin Modern glyphs,
  zoom (⌘= / ⌘- / fit width ⌘9 / actual size ⌘0), click-to-source, caret sync
  (⌘⇧J), dark preview.
- **Diagnostics** — errors, warnings and "not implemented" notes in a Problems
  panel (⌘⇧M) with go-to-source, grouped duplicates, explanations and quick
  fixes where one exists.
- **PDF export** — the bundled writer embeds Latin Modern subsets with the
  original glyph IDs and positions; no TeX installation is involved.
- **iPad companion** — pair FlashTeXPad over the local network, capture a
  sketch or photo, review the proposed LaTeX/TikZ on the Mac and insert it as
  one undoable edit.
- **Helper-process architecture** — engine, PDF writer, capture bridge, edit
  ledger and project index are separate Rust binaries with documented JSON
  Lines contracts, usable from any editor or script.

## Benchmarks

Measured on 2026-09-13 on an Apple M1 Max (MacBook Pro), macOS 26.3.1
(25D2128), this checkout at `a03b15fd`, `flashtex-render` built with
`cargo build --release` (rustc 1.99.0-nightly). The comparison oracle is
pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026, MacTeX). pdflatex never runs
inside the product; it is only used to compare.

**Full render, single document, no bibliography or images.** `HW1.tex` and
`HW2.tex` are two real problem sets (5.1 KB, 134–140 lines each, `article`
11pt with `amsmath`/`amssymb`/`enumitem`/`geometry`). Median of 5 runs, best in
parentheses; each run is a fresh process.

| Document | `flashtex-render` in-process render | `flashtex-render` process wall (`--tex`, writes PDF) | `pdflatex -interaction=batchmode` wall (one pass) |
|---|---:|---:|---:|
| `fixtures/real-world/hw1/HW1.tex` (3 pages) | 47.4 ms (46.5 ms) | 56.0 ms (54.1 ms) | 547.5 ms (536.2 ms) |
| `fixtures/real-world/hw2/HW2.tex` (3 pages in pdflatex, 4 in FlashTeX) | 46.9 ms (46.9 ms) | 56.4 ms (54.9 ms) | 551.9 ms (534.4 ms) |

Both documents render with status `recovered` (8 and 19 diagnostics for
package features that are recognised but not implemented). The pdflatex
column is a single pass with a warm font cache; a real build usually needs two
or three passes for references.

**`flashtex build` (exact-route PDF: embedded font subsets, structural
self-check).** Same machine and fixtures, this checkout at `e18f4f38`, median
of 5 fresh processes (best in parentheses), measured while several other
build jobs were running on the machine, so the render column is ~12 ms slower
than the quiet-machine figures above:

| Document | render (in-process) | exact PDF write | `flashtex build` process wall |
|---|---:|---:|---:|
| `HW1.tex` (3 pages) | 59.9 ms (59.7 ms) | 58.9 ms (58.4 ms) | 125.6 ms (124.3 ms) |
| `HW2.tex` (3 pages) | 59.8 ms (58.9 ms) | 56.3 ms (55.6 ms) | 122.6 ms (120.5 ms) |

The exact PDF write (glyph-id-preserving CFF subsets of eight faces, per-glyph
width reconciliation, the writer's structural self-check) currently costs
about as much as the layout; `flashtex-render --tex --pdf` above uses the
older, lighter text-item route.

**Edit-to-preview latency in the app** (keystroke → painted preview, release
build, programmatic typing at 30 ms intervals, engine attached through the
preview controller; from
[`docs/evidence/typing-bench-2026-09-12T102931Z.md`](docs/evidence/typing-bench-2026-09-12T102931Z.md)):

| Document | keystroke → paint p50 | p95 | p99 | compile p50 |
|---|---:|---:|---:|---:|
| `demo.tex` (6 KB) | 48 ms | 64 ms | 69 ms | 16 ms |
| 60 KB single-file body | 195 ms | 227 ms | 238 ms | 43 ms |

"Paint" is the completed CoreAnimation commit; the pixels reach the display
at the next vsync. One machine, one run per row, indicative only — the report
lists every limitation.

**App launch to first preview.** Launching the built `FlashTeX.app` binary
with `HW1.tex` as the seed document (5 fresh processes, app already on disk):
engine attached after 41–71 ms, first compile result after 166–215 ms, first
painted preview after **1.18–1.30 s** (median 1.24 s). This was measured for
this README; there is no separate evidence report for it yet.

<details>
<summary>Exact commands</summary>

```sh
# Engine (from the repository root; --font-dir points at the bundled Latin Modern faces)
cargo build --release --manifest-path crates/render-pipeline/Cargo.toml
for i in 1 2 3 4 5; do
  /usr/bin/time -p crates/render-pipeline/target/release/flashtex-render \
    --tex fixtures/real-world/hw1/HW1.tex --pdf /tmp/hw1.pdf --timing --font-dir apps/mac/Fonts
done
# CLI, exact route (the --timing line on stderr has render / pdf / total; `real` is the process wall)
cargo build --release --manifest-path crates/flashtex-cli/Cargo.toml
for i in 1 2 3 4 5; do
  /usr/bin/time -p crates/flashtex-cli/target/release/flashtex \
    build fixtures/real-world/hw1/HW1.tex -o /tmp/hw1.pdf --timing --font-dir apps/mac/Fonts
done
# The "rendered in N ms" line on stderr is the in-process time; `real` is the process wall time
# (a Python `subprocess` + `perf_counter` loop was used for the sub-10 ms wall figures above).

# Oracle (in a scratch directory so aux files do not touch the fixture)
T=$(mktemp -d) && cp fixtures/real-world/hw1/HW1.tex "$T" && cd "$T"
for i in 1 2 3 4 5; do /usr/bin/time -p /Library/TeX/texbin/pdflatex -interaction=batchmode HW1.tex >/dev/null; done

# Typing bench (app): tools/typing-bench/run.sh — see the evidence report for the exact invocation.

# Launch to first preview: run the app binary with
#   FLASHTEX_AUTOATTACH=1 FLASHTEX_NO_ACTIVATE=1 FLASHTEX_SEED_FILE=fixtures/real-world/hw1/HW1.tex FLASHTEX_LOG=/tmp/ft.log
# and time the "attached: flashtex-render", "status: revision 2: recovered" and "paint: revision 2" log lines.
```

</details>

## Installation

FlashTeX is two things — a LaTeX engine with a command line, and a native Mac
app built on it — and they install separately.

### A. Engine + CLI (macOS or Linux)

Each release ships `flashtex-cli-<version>-<platform>.tar.gz`: macOS arm64
always, Linux x86_64 best-effort (see [CI/CD](docs/ci-cd.md)). The tarball has
`bin/flashtex` plus the `flashtex-render`, `flashtex-compiler`,
`flashtex-pdf` and `flashtex-pdf-exact` helpers, with the fonts and metrics
they need in `share/flashtex/`.

```sh
curl -fsSL https://flash-tex.github.io/flashtex/install-cli.sh | sh
```

Detects your OS/CPU, verifies the download against the release's
`SHA256SUMS` (refuses on a mismatch), and installs into `~/.local/bin` +
`~/.local/share/flashtex` — pass `--prefix /usr/local` for a system install,
`--version vX.Y.Z` for another release, or `--uninstall` to remove it. Or
extract the tarball yourself and run `bin/flashtex build main.tex` directly;
`bin/flashtex install-cli` links it into `/usr/local/bin`.

### B. Engine + native GUI (macOS only, for now)

Requirements: **macOS 14 Sonoma or later on Apple Silicon** (arm64). No TeX
installation needed — the app bundles the engine, Latin Modern fonts and TeX
metrics, plus this same CLI at `Contents/MacOS/flashtex-cli`.

```sh
curl -fsSL https://flash-tex.github.io/flashtex/install.sh | sh
```

Downloads the pinned release DMG, verifies its SHA-256, installs into
`/Applications` (or `~/Applications`) and strips the quarantine flag. Or
download `FlashTeX.dmg` from
[Releases](https://github.com/flash-tex/flashtex/releases) and drag FlashTeX
into Applications yourself — the app is ad-hoc signed, not notarized, so the
first time: **right-click → Open** and confirm.

Other platforms aren't ruled out, just not built yet: the GUI is SwiftUI today
(macOS only), but the CLI already runs anywhere it's built for, and any editor
or CI can drive the engine over the documented JSON Lines `worker` protocol
(see [Extending FlashTeX](docs/extensibility.md)).

**From source** (either path; Xcode Command Line Tools with Swift 6, stable
Rust from rustup — there is no root Cargo workspace, each crate builds on its
own):

```sh
git clone https://github.com/flash-tex/flashtex.git && cd flashtex
cargo build --release --manifest-path crates/flashtex-cli/Cargo.toml   # A: the engine + CLI
scripts/ci/build-helpers.sh                 # release-builds the CLI and every helper crate
apps/mac/scripts/make-app.sh --install      # B: packages FlashTeX.app into ~/Applications
apps/mac/scripts/make-app.sh --dmg          # or: build a disk image
```

Full details: [Getting started](docs/user/README.md) and
[Building from source](docs/user/compiler.md#building-from-source).

## Quick start — CLI

```sh
flashtex build main.tex                       # main.pdf next to it; \input/\include resolved from its folder
flashtex build main.tex -o out.pdf --timing   # exact-route PDF; render / pdf / total on stderr
flashtex check main.tex --json                # diagnostics only, flashtex-check/1 on stdout
flashtex watch main.tex                       # rebuild on every change in the project; Ctrl-C stops
flashtex supported                            # implemented LaTeX + coverage (--json, --md)
flashtex fonts                                # which fonts and TeX metrics this binary resolves
```

Diagnostics are `file:line:col: severity[code] message` lines on stderr with
a summary line; exit `0` when the document rendered (`ok`, or `recovered`
with diagnostics — `--strict` makes recovered errors exit `1`), `1` when it
`failed`, `2` for a usage error. The app bundle and the tarball find their
fonts next to the binary; a source build adds `--font-dir apps/mac/Fonts`.
`flashtex worker` is the long-running JSON Lines worker the IDE drives (one
`compile` request per stdin line, one `compile_result` per stdout line), so
any editor or CI can embed the engine. Every flag, the JSON schema, the font
search order and the helper binaries are documented in
[The `flashtex` command line](docs/user/compiler.md).

## Quick start — IDE

1. **Open FlashTeX**, then *File › Open LaTeX File…* (⌘O) — or type into the
   sample buffer and ⌘⇧S to save it. The file becomes the entry document and
   its folder the project root; the bundled engine attaches automatically.
2. **Type.** Auto-compile is on (⌘B compiles on demand); zoom the preview with
   ⌘= / ⌘-, click any word to jump to its source, open **Problems** (⌘⇧M) to
   see and fix diagnostics.
3. **Export** with *File › Export PDF (exact, v2)…*; **pair an iPad** with
   *Edit › Nearby Companion…* (⌘⇧N) → Advertise → Show Pairing Code, then scan
   the code from FlashTeXPad.

Everything else — projects, IntelliSense, capture conversion, preferences and
the full shortcut table — is in [The Mac app](docs/user/gui.md); the
companion is in [FlashTeXPad for iPad](docs/user/ipad.md).

## Architecture

The engine is a library (`crates/render-pipeline`) with one front end,
`flashtex` (`crates/flashtex-cli`). The Mac app never links it: it launches
`flashtex worker` (or the bare `flashtex-render`) and talks to it over
stdin/stdout with versioned JSON Lines contracts
([runtime-v1](docs/contracts/runtime-v1.md), the rendering-v2 display list in
[`protocol/`](protocol/)); a crashed helper is relaunched and the last good
preview stays on screen. The same contracts make the engine usable from other
editors and CI.

```
crates/
  flashtex-cli/      the `flashtex` command line: build / check / watch / supported / worker / fonts
  compiler/          lexer, LaTeX subset, runtime-v1 JSON Lines worker (flashtex-compiler)
  render-pipeline/   the engine: parse tree → styled blocks → shaping → Knuth-Plass →
                     Appendix G math → pages → display list (flashtex-render)
  font-engine/, font-resources/, math-layout/, paragraph-layout/, document-style/, …
  pdf/               PDF writer (flashtex-pdf, flashtex-pdf-exact)
  bridge/            iPad capture receipt, conversion, reviewed edits (flashtex-bridge)
  project-files/, edit-ledger/, project-index/, preview-controller/, …
apps/mac/            the SwiftUI/AppKit IDE, bundled fonts and TeX metrics, packaging scripts
apps/ios/            FlashTeXPad, the iPad capture companion
protocol/            rendering-v2 schema and wire fixtures
fixtures/            real-world documents with pdfLaTeX reference PDFs
docs/                user guides, contracts, evidence reports
```

## Community

Questions, feedback and release news: the FlashTeX Discord — <https://discord.gg/J4kHDJmTrD>.

## Contributing

Issues and pull requests: <https://github.com/flash-tex/flashtex>. CI builds
and tests every crate, the Mac app and the iPad companion on each push; tagging
`vX.Y.Z` builds the DMG and CLI tarballs and publishes a release — see
[CI/CD](docs/ci-cd.md). Agents (Claude Code, Codex, Cursor) working in this
repository must follow [AGENTS.md](AGENTS.md); the onboarding notes are in
[docs/agents/README.md](docs/agents/README.md).

## License

[MIT](LICENSE) © 2026 flash-tex. The bundled Latin Modern and New Computer
Modern fonts are under the [GUST Font License](apps/mac/Fonts/GUST-FONT-LICENSE.TXT)
(see [apps/mac/Fonts/README.md](apps/mac/Fonts/README.md)).
