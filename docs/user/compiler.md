# The `flashtex` command line, building from source, and supported LaTeX

FlashTeX is a LaTeX engine first: `flashtex` is the compiler on the command
line, and the Mac IDE is a complementary app that drives the same engine. No
TeX distribution is involved anywhere — parsing, layout, fonts and the PDF
writer are all original Rust code, linked into one binary.

```sh
flashtex build main.tex                 # main.pdf next to it; \input/\include resolved from its folder
flashtex build main.tex -o out.pdf --timing
flashtex check main.tex --json          # diagnostics only, machine-readable
flashtex watch main.tex                 # rebuild on every change; Ctrl-C stops
flashtex supported                      # what LaTeX is implemented, with coverage
flashtex fonts                          # which fonts/metrics this binary resolves
flashtex --version
```

## Install

This is the engine + CLI, independent of the Mac app — it runs on macOS or
Linux, with no TeX installation:

```sh
curl -fsSL https://flash-tex.github.io/flashtex/install-cli.sh | sh
```

Detects your OS/CPU, downloads the matching
`flashtex-cli-<version>-<platform>.tar.gz` from
[Releases](https://github.com/flash-tex/flashtex/releases), verifies it
against that release's `SHA256SUMS` (refuses on a mismatch), and installs
`flashtex` into `~/.local/bin` plus its fonts/metrics into
`~/.local/share/flashtex`. Flags: `--prefix DIR` (e.g. `/usr/local`, may need
sudo), `--version vX.Y.Z` (default: the latest release), `--uninstall`. For
the Mac app instead (SwiftUI GUI, bundles this same CLI), see
[Getting started](README.md#install).

Where it lives:

| Install | Path |
|---|---|
| `install-cli.sh`, or the CLI tarball extracted by hand (`flashtex-cli-<version>-<platform>.tar.gz` from [Releases](https://github.com/flash-tex/flashtex/releases)) | `bin/flashtex`, fonts and metrics in `share/flashtex/` — extract anywhere; `bin/flashtex install-cli` links it into `/usr/local/bin` |
| Mac app | `/Applications/FlashTeX.app/Contents/MacOS/flashtex-cli` (uses the app's `Contents/Resources/{Fonts,texmf}`) |
| Source build | `crates/flashtex-cli/target/release/flashtex` — add `--font-dir apps/mac/Fonts` and `FLASHTEX_TFM_DIRS=apps/mac/Fonts/texmf/fonts/tfm/public/lm`, or install the tarball layout ([Fonts](#fonts-and-metrics)) |

The older helpers (`flashtex-render`, `flashtex-compiler`, `flashtex-pdf`,
`flashtex-pdf-exact`) still ship next to it and are described
[below](#helper-binaries); `flashtex` is the same engine with a proper front
end, and its `worker` subcommand is byte-for-byte the protocol the IDE speaks.
Any editor or CI can drive that same JSON Lines protocol directly — see
[Extending FlashTeX](../extensibility.md).

## Subcommands

```
flashtex build [<main.tex>|<dir>] [-o out.pdf] [--project-root DIR] [--font-dir DIR]...
               [--font ROLE=NAME]... [--v2 out.json] [--timing] [--verbose] [--strict] [--json] [-j N]
               [--fetch ask|always|never] [--write-pins]
flashtex check [<main.tex>|<dir>] [--json] [--strict] [--project-root DIR] [--font-dir DIR]... [--fetch ask|always|never]
flashtex watch [<main.tex>|<dir>] [-o out.pdf] [--project-root DIR] [--font-dir DIR]... [--interval MS]
               [--fetch ask|always|never] [--write-pins]
flashtex manifest init [<main.tex>|<dir>] [--force]
flashtex manifest show [<main.tex>|<dir>] [--json]
flashtex packages list [--json]
flashtex packages fetch <name>... [<main.tex>|<dir>]
flashtex packages clear [<name>]
flashtex supported [--json|--md|--coverage]
flashtex worker [--font-dir DIR]... [--project-root DIR] [--v2 out.json] [--pdf out.pdf] [--timing]
flashtex fonts [--font-dir DIR]... [--json]
flashtex install-cli [DIR]
flashtex --version | --help
```

### `build`

Typesets a project and writes a PDF. The entry is a `.tex` file, a
directory, or nothing (the current directory): a directory needs a
[`flashtex.toml`](project-manifest.md) naming `[project] entry`, or exactly
one `.tex` file in it — two or none is a usage error, never a guess. The
entry file's directory is the **project root** — the manifest's directory
when there is one — unless `--project-root` says otherwise; every `\input` and
`\include` is resolved from that root (project-relative, `.tex` appended when
missing), read through a rooted directory handle so no include can reach
outside the root through `..` or a symlink, and cycles and missing files are
diagnostics, never crashes. Images (`\includegraphics`) resolve under the
same root.

The PDF is the **exact route** — the display list the engine produces is
written by the pdf crate's exact writer: Latin Modern (and New Computer
Modern Math) font programs embedded as glyph-id-preserving CFF subsets, the
engine's own advances, typed rules, images and hyperlinks. It is the same
output as *File › Export PDF* in the app. The file is written atomically
(temporary sibling + rename), so a viewer never sees a torn PDF.

| Flag | Meaning | Default |
|---|---|---|
| `-o`, `--output FILE` | The PDF to write | `<main>.pdf` next to the entry file, or in the manifest's `[project] output` directory when set |
| `--project-root DIR` | Root that includes and images resolve under; the entry must be inside it | the manifest's directory, else the entry file's |
| `--font-dir DIR` | Extra font directory, probed first (repeatable) | — |
| `--v2 FILE` | Also write the rendering-v2 `display_list` envelope (images included) — the input of `flashtex-pdf-exact from-v2` and of *File › Open Display List (v2)…* | off |
| `--timing` | One line on stderr: `render N ms (P passes), pdf N ms, total N ms` | off |
| `-v`, `--verbose` | Also print the PDF route's notes: every embedded font, subset size, width adjustments | off |
| `--strict` | Exit 1 when any **error** diagnostic was reported, even though the document rendered (`recovered`); the PDF is still written | off |
| `--json` | Also print the [`flashtex-check/1`](#the-json-report) report on stdout | off |
| `-j`, `--jobs N` | Accepted for build-system compatibility; the engine is single-threaded | — |
| `--class-options OPTS` | Class options assumed when the source has no `\documentclass` (body-only input) | `12pt` |
| `--secnumdepth N` | Section numbering depth when the source does not set the counter | `2` |
| `--font ROLE=NAME` | The installed family for a role — `text`, `sans`, `mono` or `math` (repeatable). Outranks the manifest's [`[fonts]`](project-manifest.md#keys) table for that role; the document's own `\setmainfont` still wins | the manifest, else the class fonts |
| `--fetch POLICY` | `ask`, `always` or `never`: whether a package the documents ask for that is not in the project, a [local library](project-manifest.md#packages) or the package cache may be fetched from the manifest's `[packages] source`. `ask` prompts once per package when stdin and stderr are terminals and is `never` with a `package_fetch` diagnostic otherwise | the manifest's `fetch`; without a manifest nothing is resolved |
| `--write-pins` | (build/watch) Record each version fetched during this build in the manifest's `[packages] pin` table; without it a build never rewrites the manifest | off |

### `check`

`build` without output files: the same discovery, parse and layout (page
count and every diagnostic depend on layout), diagnostics on stderr and,
with `--json`, the report on stdout. `-o`/`--v2` are rejected.

### `watch`

Builds once, then polls the whole project closure (every `.tex`, `.bib` and
image the graph reaches, re-discovered after each build so new includes are
picked up) every `--interval` milliseconds (default 250, minimum 20) and
rebuilds when a size or modification time changes, printing which files
changed, the rebuild number and the timing line each time. Ctrl-C stops it;
because outputs are written atomically nothing is left half-written.

### `manifest`

`flashtex manifest init [<main.tex>|<dir>] [--force]` writes the commented
[`flashtex.toml`](project-manifest.md) template next to the entry (naming
the actual entry: the file given, or the directory's only `.tex`), refusing
to overwrite an existing one unless `--force`. `flashtex manifest show
[<main.tex>|<dir>] [--json]` prints the manifest that governs the entry —
the defaults when there is none — with each `texinputs` entry classified
(`inside` the root, `outside` and mounted at `texinputs/<i>/`, or
`invalid` with the reason) and every warning; `--json` is the
`flashtex-manifest/1` document (`entry`, `path`, `exists`, `manifest`,
`texinputs`, `warnings`).

### `packages`

The per-user package cache ([where and how it is laid out](project-manifest.md#packages);
`FLASHTEX_PACKAGE_CACHE` overrides the directory). `flashtex packages list
[--json]` prints every cached package with its version, files, source URL
and fetch time (`--json`: `flashtex-packages/1`). `flashtex packages fetch
<name>... [<main.tex>|<dir>]` fetches each name from the source the
governing manifest names (the one for the entry or directory given, else
the current directory's; CTAN without one), honouring its `pin` — the
command itself is the consent, so `fetch = "ask"`/`"never"` do not apply,
`source = "none"` still does; a package that ships `.ins`/`.dtx` sources is
unpacked with FlashTeX's docstrip and each generated file is printed with
the batch file and sources it came from (docstrip's notes follow on
stderr, the first five; all are in the cache's `manifest.json`); a package
CTAN does not have, or whose sources yield no `.sty`/`.cls`/… (needs
docstrip), exits 1 with the reason. `flashtex packages clear [<name>]`
removes one package (every version) or the whole cache.

### `supported`

The implemented-LaTeX inventory of the compiler linked into this binary
(`flashtex_compiler::supported`), so it cannot drift from what `build`
accepts. Plain `flashtex supported` prints command/environment counts, the
coverage of the canonical inventory (46% at the time of writing) and one
line per package set; `--json` is the `flashtex-supported-latex/1` document
(the same as `crates/compiler/supported/supported-latex.json`), `--md` the
reference page reproduced [below](#supported-latex), `--coverage` the
coverage table alone.

### `worker`

The runtime-v1 JSON Lines worker: one `compile` request per stdin line, one
`compile_result` per stdout line, followed by the `display_list` line when
the client negotiated `display-list-v2`; a block cache across requests makes
a keystroke retypeset only the paragraph it touched. This is what the IDE
launches (`FLASHTEX_COMPILER` may point at `flashtex worker` or at
`flashtex-render`; they share the loop). `--v2`/`--pdf` write the last
request's display list / exact PDF as side outputs. The wire format is
[runtime-v1](../contracts/runtime-v1.md) and
[runtime-v1-display-list-v2](../contracts/runtime-v1-display-list-v2.md).

### `fonts`

Prints the font and TFM search lists in order, marking which directories
exist, the `FLASHTEX_*` overrides in effect, whether the Latin Modern text
and math faces were found and whether the pinned Latin Modern 2.004 metric
set loads; exit 1 when either is missing. `--json` is the
`flashtex-fonts/1` object with the same fields.

### `install-cli`

Creates the symlink `DIR/flashtex` (default `/usr/local/bin`) to the running
binary — a link, not a copy, so the binary keeps finding its
`share/flashtex` (tarball) or `Contents/Resources` (app) fonts. Refuses to
replace an unrelated file and explains when the directory needs `sudo`.

## Diagnostics and exit status

Every diagnostic is one stderr line:

```
file:line:col: severity[code] message (recovery: what was rendered instead)
```

`file` is project-relative (`sections/intro.tex`); `line:col` are 1-based
(column in characters) and are omitted when a diagnostic has no source
position (font resource notes, unstable labels). A summary line follows:

```
flashtex: main.tex: recovered, 3 pages, 0 errors, 6 warnings -> pdf main.pdf
```

| Status | Meaning | Exit |
|---|---|---|
| `ok` | No diagnostics | 0 |
| `recovered` | Diagnostics were reported (an unsupported command, a missing include, a warning) but the document rendered around them; the PDF is written | 0, or 1 with `--strict` when any of them is an error |
| `failed` | Errors and no content; no PDF | 1 |
| usage | Unknown option, unreadable entry, entry outside `--project-root`, unwritable output | 2 |

The codes (`compiler`, `overfull_hbox`, `math_limitation`, `missing_file`,
`include_cycle`, `path_escapes_root`, …) are listed under
[Troubleshooting](#troubleshooting).

### The JSON report

`check --json` and `build --json` print one JSON object on stdout, schema
`flashtex-check/1`. The keys below are stable; new keys may be added.

```json
{
  "schema": "flashtex-check/1",
  "version": "flashtex 0.1.0 (e18f4f38a1c2)",
  "entry": "main.tex",
  "project_root": "/abs/path/to/project",
  "documents": ["main.tex", "sections/intro.tex"],
  "status": "recovered",
  "pages": 3,
  "diagnostics": [
    {"path": "sections/intro.tex", "line": 13, "column": 1,
     "start_byte": 412, "end_byte": 430,
     "severity": "warning", "code": "compiler",
     "message": "\\foo is not supported by this compiler version",
     "recovery": "skipped the command and continued"}
  ],
  "summary": {"errors": 0, "warnings": 1},
  "timing": {"render_ms": 47.2, "total_ms": 106.9, "passes": 1},
  "outputs": {"pdf": "/abs/path/to/main.pdf"}
}
```

`line`, `column`, `start_byte`, `end_byte` and `recovery` are `null` when
absent; `severity` is `error` or `warning`; `outputs` is empty for `check`.

## Multi-file projects

```
paper/
  main.tex          \input{sections/intro}  \include{sections/method}
  sections/intro.tex
  sections/method.tex
  figures/plot.png  \includegraphics{figures/plot}
```

`flashtex build paper/main.tex` uses `paper/` as the root, sends `main.tex`
plus both sections to the engine (entry first, depth-first order — the
`documents` list of the report and of the `--v2` envelope), and attributes
each diagnostic to the file it came from. A [`flashtex.toml`](project-manifest.md)
in `paper/` can name the entry (so `flashtex build paper` works) and add
`texinputs` directories whose `.sty`/`.cls`/`.tex`/`.bib`/`.def`/`.clo`
files follow the closure in the `documents` list; without one, nothing
changes. `--project-root` may be a parent
directory when the entry lives in a subfolder (`flashtex build
--project-root . paper/main.tex`); a path that escapes the root, a cycle or a
missing file is an error diagnostic (`path_escapes_root`, `include_cycle`,
`missing_file`) and the build continues without it. Bibliographies are
resolved by the engine from `thebibliography`; `.bib` files are discovered
(and watched) but BibTeX is not run.

## Fonts and metrics

The engine lays text out with the same TeX font metrics pdfLaTeX uses
(`ec-lm*.tfm`, `rm-lmr*.tfm`, digest-bound to Latin Modern 2.004) and paints
and embeds the Latin Modern OpenType faces (plus New Computer Modern Math for
`\mathbb`, `\mathcal` and the `amssymb` glyphs). It looks for them, in order
(`flashtex fonts` prints the resolved lists):

1. `FLASHTEX_FONT_DIRS` / `FLASHTEX_TFM_DIRS` (colon-separated; explicit
   entries always win) and `--font-dir`;
2. `FLASHTEX_LM_DIR`;
3. relative to the executable: the app bundle's
   `<exe>/../Resources/{texmf,Fonts}`, a sibling `<exe>/{texmf,Fonts}`, then
   the tarball's `<exe>/../share/flashtex/{texmf,Fonts}` — so both the app
   and the extracted tarball work with **no TeX installation and no
   environment**;
4. MacTeX/BasicTeX 2025–2026 and Debian TeX Live paths.

Without the TFMs the OpenType metrics are used and a `tfm_missing` /
`math_metrics_opentype` warning says so; a missing required metric set is the
blocking `required_metrics_unavailable` error, never a silent fallback.

A document can choose its own fonts the way it would under XeLaTeX or
LuaLaTeX. Text: `\setmainfont`/`\setsansfont`/`\setmonofont`,
`\newfontfamily`, `\fontspec` (fontspec's syntax; `Scale=`, `BoldFont=`,
`ItalicFont=`, `Numbers=OldStyle`, `Ligatures=TeX` honoured) or the
manifest's `[fonts]` table, over every font installed on the machine.
Math: `\setmathfont{Family}` in the preamble, `\usepackage{unicode-math}`
alone (Latin Modern Math) or `[fonts] math = "…"` set every formula from
that face's OpenType `MATH` table with LuaTeX's rules for it — the table's
constants, cut-in kerns, top-accent anchors, glyph assemblies for tall
delimiters, the face's script sizes and `ssty` forms. `\mathcal`,
`\mathfrak` and `\mathbb` come from the math face; `\mathbf`, `\mathsf`,
`\mathit`, `\mathtt` and `\mathrm` from the text fonts, as unicode-math sets
them. `flashtex-render --list-math-fonts` names the usable families; one
without a `MATH` table, or not installed, is a `math_font_unavailable`
warning and the formula keeps TeX's metrics. A document that names no font
is laid out exactly as before, with pdfLaTeX's metrics. The
bundle covers Latin Modern Roman regular/bold/italic at 5–17 pt, the math
faces, and — despite older notes here — sans (`lmsans*`, including demi-condensed),
slanted (`lmromanslant*`/`lmmonoslant*`), small caps (`lmromancaps*`/
`lmmonocaps*`) and typewriter (`lmmono*`, including bold via `lmmonolt*`):
`\textsf`/`\texttt`/`\textsl`/small caps all resolve to real bundled faces
(`crates/render-pipeline/src/fonts.rs`'s `latin_modern_file`), not a
substitution. The PDF writer resolves each face by content hash in the same
directories, so the embedded program is exactly the file the layout used.

## Helper binaries

| Tool | What it is |
|---|---|
| `flashtex-render` | The engine as a bare runtime-v1 worker (`flashtex worker` is the same loop). `--tex FILE` is a one-shot mode: single file, no include resolution. `--v2 out.json` writes the display list of the last render — complete, with no reply-line limit. `--pdf out.pdf` writes that render through the exact route, the same bytes `flashtex build` and the Mac app's Export PDF produce. |
| `flashtex-compiler` | The original FT-002 worker with Core-14 (Times) metrics and a smaller LaTeX subset; same wire protocol, no flags. `--supported [json|markdown|coverage]` prints the inventory (`flashtex supported` is the same data). *File › Attach Built Compiler* (⌘⇧K) attaches it. |
| `flashtex-pdf` | `flashtex-pdf [INPUT.json] --out OUTPUT.pdf [--verify] [--embed-font PATH|auto] [--default-face embedded|lm|times]` — a PDF from a runtime-v1 `compile_result` (text items and rules); characters outside WinAnsi/Symbol/the embedded face become `?`. No longer reachable from the Mac app: the app's one export route is `flashtex-pdf-exact`. |
| `flashtex-pdf-exact` | `from-v2 LIST.json --out OUT.pdf [--font-dir DIR]... [--project-root DIR]` is the exact route as a separate step (what `flashtex build` runs in-process); `reemit`, `classify` and `dump` are PDF-comparison utilities used by the fidelity tests. |

## Building from source

Requirements: a stable Rust toolchain from rustup (the CLI builds on macOS
and Linux); for the Mac app, macOS 14+ on Apple Silicon and Xcode Command
Line Tools (Swift 6). There is **no Cargo workspace** at the repository root
— build each crate on its own:

```sh
git clone https://github.com/flash-tex/flashtex.git && cd flashtex
cargo build --release --manifest-path crates/flashtex-cli/Cargo.toml   # the CLI (links the engine)
crates/flashtex-cli/target/release/flashtex build fixtures/real-world/hw1/HW1.tex \
  --font-dir apps/mac/Fonts -o /tmp/hw1.pdf --timing
scripts/ci/build-helpers.sh                          # every helper the Mac app bundles, release mode
apps/mac/scripts/make-app.sh --install --open        # packages FlashTeX.app into ~/Applications
scripts/ci/package-cli.sh 0.0.0-local macos-arm64 dist   # the tarball, from the built binaries
```

`crates/flashtex-cli` depends on `crates/render-pipeline` by path, which
builds against the pinned sibling mirrors under
`crates/render-pipeline/vendor/` (see `vendor/VENDORING.md`). Binaries land
in `crates/<crate>/target/release/`. `make-app.sh` bundles `flashtex` as
`Contents/MacOS/flashtex-cli` alongside the helpers, verifies the bundled fonts
and metrics against a pinned manifest, and ad-hoc signs the bundle; add
`--dmg` for a disk image. `cargo test --release` in `crates/flashtex-cli`
builds HW1/HW2 and the multi-file fixture end to end (including a run with
the host TeX trees denied by `sandbox-exec` on macOS); `swift test` in
`apps/mac` runs the app's tests.

The section below is generated from the compiler itself
(`flashtex-compiler --supported markdown`, identical to `flashtex supported
--md`). Do not edit it by hand: change the compiler, then run
`crates/compiler/scripts/render_supported_latex.sh`.

<!-- BEGIN GENERATED supported-latex: `flashtex-compiler --supported markdown`; do not edit by hand -->
## Supported LaTeX

This compiler implements a finite LaTeX subset: 499 text-mode and 679 math-mode command entries, 87 environments and 42 layout-neutral packages. Every other command produces an explicit "not supported" diagnostic naming it, and every other environment or package a warning; nothing is dropped silently. Descriptions note approximations. Outstanding features with reproductions are in `crates/compiler/UNSUPPORTED.md`.

Regenerate with `crates/compiler/scripts/render_supported_latex.sh`; `cargo test --test supported_latex` fails when this section is stale.

### Coverage

| Canonical set | Commands supported | Environments supported |
| --- | ---: | ---: |
| kernel | 226/410 (55.1%) | 22/30 (73.3%) |
| amsmath | 47/102 (46.1%) | 17/21 (81.0%) |
| amssymb | 225/229 (98.3%) | none defined |
| enumitem | 1/17 (5.9%) | none defined |
| geometry | 0/7 (0.0%) | none defined |
| graphicx | 6/8 (75.0%) | none defined |
| hyperref | 3/132 (2.3%) | 0/2 (0.0%) |
| tikz | 0/43 (0.0%) | 0/2 (0.0%) |
| xcolor | 14/71 (19.7%) | none defined |
| siunitx | 20/240 (8.3%) | none defined |
| **total** | **581/1314 (44.2%)** | |

Generated by `flashtex-compiler --supported coverage` against `crates/compiler/supported/canonical-latex.tsv` (LaTeX2e reference-manual index and package sources, each name confirmed by pdfLaTeX; TeX Live 2026). The total row counts commands and environments together. Supported means handled without an unsupported diagnostic, not typographic parity.

**The denominator excludes math-mode symbol commands entirely.** `canonical-latex.tsv`'s candidates are `@findex`/`@EnvIndex` entries from the LaTeX2e reference manual plus each listed package's own source files (see `crates/compiler/scripts/canonical_latex.py`'s header); math symbols such as `\alpha` and `\odot` are neither in that manual's index nor in any of the nine package source lists, so they never become candidates and are absent from the table. The compiler tracks math-command support separately (`crates/compiler/src/math.rs`'s `COMMAND_GLYPHS`/`OPERATOR_NAMES`, `crates/compiler/src/supported.rs`'s `Origin::MathSymbol`/`MathOperator`/`MathStructure`; the full list is in `docs/user/compiler.md`'s math tables), but `--supported coverage` does not count them.

Canonical sources:

- Engine: TeX 3.141592653 (TeX Live 2026)
- Source kernel: doc/latex/latex2e-help-texinfo/latex2e.texi (UPDATED May 2024) @findex commands and @EnvIndex environments; confirmed with LaTeX2e 2025-11-01 patch level 0, article class
- Source amsmath: tokens of amsmath.sty, amstext.sty, amsbsy.sty, amsopn.sty (amsmath.sty 2025/07/09 v2.17z AMS math features; amstext.sty 2024/11/17 v2.01 AMS text; amsbsy.sty 1999/11/29 v1.2d Bold Symbols; amsopn.sty 2022/04/08 v2.04 operator names); dependency baseline: amsgen
- Source amssymb: tokens of amssymb.sty, amsfonts.sty (amssymb.sty 2013/01/14 v3.01 AMS font symbols; amsfonts.sty 2013/01/14 v3.01 Basic AMSFonts support); dependency baseline: none
- Source enumitem: tokens of enumitem.sty (enumitem.sty 2025/02/06 v3.11 Customized lists); dependency baseline: none
- Source geometry: tokens of geometry.sty (geometry.sty 2026/03/07 v6.0 Page Geometry); dependency baseline: keyval, ifvtex, iftex
- Source graphicx: tokens of graphicx.sty, graphics.sty (graphicx.sty 2024/12/31 v1.2e Enhanced LaTeX Graphics (DPC,SPQR); graphics.sty 2024/08/06 v1.4g Standard LaTeX Graphics (DPC,SPQR)); dependency baseline: keyval, trig
- Source hyperref: tokens of hyperref.sty, nameref.sty (hyperref.sty 2026-01-29 v7.01p Hypertext links for LaTeX; nameref.sty 2026-01-29 v2.58 Cross-referencing by name of section); dependency baseline: iftex, keyval, kvsetkeys, kvdefinekeys, pdfescape, ltxcmds, pdftexcmds, infwarerr, hycolor, refcount, gettitlestring, kvoptions, etoolbox, stringenc, intcalc, url, bitset, bigintcalc, rerunfilecheck, uniquecounter
- Source tikz: tokens of tikz.sty, tikz.code.tex (tikz.sty 2025-08-29 v3.1.11a (3.1.11a); pgf 3.1.11a); dependency baseline: pgf, pgfrcs, pgfcore, graphicx, keyval, graphics, trig, pgfsys, xcolor, pgfcomp-version-0-65, pgfcomp-version-1-18, pgffor, pgfkeys, pgfmath
- Source xcolor: tokens of xcolor.sty (xcolor.sty 2024/09/29 v3.02 LaTeX color extensions (UK)); dependency baseline: none
- Source siunitx: tokens of siunitx.sty (siunitx.sty 2025-07-09 v3.4.14 A comprehensive (SI) units package); dependency baseline: translations, etoolbox, pdftexcmds, infwarerr, iftex, ltxcmds, amstext, amsgen, array

### Text commands

| Command | Arguments | Behaviour |
| --- | --- | --- |
| `\num` | `[options]{number}` | siunitx number: digit groups, decimal marker, exponent, uncertainty, as an upright formula |
| `\qty` | `[options]{number}{units}` | siunitx quantity: number, unbreakable thin space, unit |
| `\unit` | `[options]{units}` | siunitx unit: prefixes, powers, \per as a power, fraction or solidus; literal m/s |
| `\si` | `[options]{units}` | siunitx v2 name of \unit |
| `\SI` | `[options]{number}[pre-unit]{units}` | siunitx v2 name of \qty with an optional pre-unit |
| `\numlist` | `[options]{numbers}` | siunitx list of ;-separated numbers joined by list-separator and " and " |
| `\numrange` | `[options]{number}{number}` | siunitx range: two numbers joined by range-phrase " to " |
| `\qtylist` | `[options]{numbers}{units}` | siunitx list of quantities, the unit repeated |
| `\qtyrange` | `[options]{number}{number}{units}` | siunitx range of quantities, the unit repeated |
| `\SIlist` | `[options]{numbers}{units}` | siunitx v2 name of \qtylist |
| `\SIrange` | `[options]{number}{number}{units}` | siunitx v2 name of \qtyrange |
| `\ang` | `[options]{degrees;minutes;seconds}` | siunitx angle with degree, minute and second marks |
| `\sisetup` | `{options}` | siunitx settings for the following commands (document-global in this model) |
| `\DeclareSIUnit` | `[options]{\name}{units}` | defines a siunitx unit macro usable inside \unit and \qty |
| `\section` | `{...}` | numbered section heading; starred form unnumbered |
| `\subsection` | `{...}` | numbered subsection heading; starred form unnumbered |
| `\subsubsection` | `{...}` | numbered subsubsection heading; starred form unnumbered |
| `\paragraph` | `{...}` | run-in heading: bold, flush, set into the first line of the paragraph that follows it |
| `\subparagraph` | `{...}` | run-in heading indented by \parindent, set into the first line of the paragraph that follows it |
| `\tableofcontents` |  | article contents list from the previous layout pass; in beamer a frame's sections and subsections, with the [currentsection], [currentsubsection], [hideallsubsections], [hideothersubsections] and [sectionstyle=..]/[subsectionstyle=..] options |
| `\index` | `{entry}` | makeidx index entry (\|modifier, @sort key and !subentry live inside the braces): accepted, never typeset (no indexing backend) |
| `\glossary` | `{entry}` | glossary entry: accepted, never typeset (no glossary backend) |
| `\textbf` | `{...}` | bold text |
| `\textmd` | `{...}` | medium-weight text |
| `\emph` | `{...}` | emphasis: toggles italic |
| `\textit` | `{...}` | italic text |
| `\textsl` | `{...}` | slanted text (typeset as italic) |
| `\textsc` | `{...}` | small capitals (upright in the Core 14 layout) |
| `\textup` | `{...}` | upright text |
| `\texttt` | `{...}` | typewriter text |
| `\textrm` | `{...}` | roman text |
| `\textsf` | `{...}` | sans-serif text |
| `\textnormal` | `{...}` | normal text face |
| `\begin` | `{env}` | opens a supported environment |
| `\end` | `{env}` | closes the innermost open environment |
| `\par` |  | ends the paragraph |
| `\documentclass` | `[options]{class}` | records the class and its 10pt/11pt/12pt size option; only the document body is typeset |
| `\NeedsTeXFormat` | `{format}[date]` | accepted no-op; the format requirement is metadata with no visible output |
| `\ProvidesClass` | `{name}[release]` | accepted no-op; a .cls declaration with no visible output |
| `\ProvidesPackage` | `{name}[release]` | accepted no-op; a .sty declaration with no visible output |
| `\ProvidesFile` | `{name}[release]` | accepted no-op; a file declaration with no visible output |
| `\DocumentMetadata` | `{keys}` | diagnosed: PDF metadata keys have no effect here; an error after \documentclass |
| `\setlength` | `{\length}{dimension}` | preamble page geometry and \parskip; \parindent of 0pt; other lengths warn |
| `\addtolength` | `{\length}{dimension}` | preamble page geometry and \parskip; accumulates onto the current value |
| `\usepackage` | `[options]{a,b,c}` | records packages; layout-neutral ones are silent, every other package warns that it is not implemented |
| `\definecolor` | `[class]{name}{model}{spec}` | colour definition in rgb, cmy, cmyk, gray, RGB, HTML or Gray (model lists pick the target model) |
| `\providecolor` | `[class]{name}{model}{spec}` | \definecolor unless the colour is already defined |
| `\xdefinecolor` | `[class]{name}{model}{spec}` | xcolor synonym of \definecolor |
| `\colorlet` | `[class]{name}[model]{expression}` | names an xcolor expression, optionally converted to a model |
| `\definecolorset` | `[class]{models}{head}{tail}{set}` | defines name,spec;... colours in one go |
| `\DefineNamedColor` | `{named}{name}{model}{spec}` | driver named colour, as dvipsnam.def uses it |
| `\selectcolormodel` | `{model}` | xcolor target model: natural, rgb, cmy, cmyk or gray |
| `\color` | `[model]{expression}` | text colour for the rest of the group; pdfTeX's exact operator values |
| `\textcolor` | `[model]{expression}{text}` | text in a colour |
| `\pagecolor` | `[model]{expression}` | page background colour, document-wide |
| `\nopagecolor` |  | removes the page background colour |
| `\normalcolor` |  | back to the default text colour |
| `\colorbox` | `[model]{expression}{text}` | text on a filled box \fboxsep larger than its content |
| `\fcolorbox` | `[model]{frame}{fill}{text}` | \colorbox inside a \fboxrule frame |
| `\newcolumntype` | `{X}[n]{spec}` | array column type expanded in later tabular specifications |
| `\arraybackslash` |  | array no-op: \\ already ends the row inside p, m and b entries |
| `\arrayrulecolor` | `[model]{colour}` | colortbl: colour of later table rules |
| `\doublerulesepcolor` | `[model]{colour}` | colortbl: colour of the gap between double rules |
| `\setlist` | `*[list]{options}` | enumitem keys recorded on every matching list; itemsep and topsep also set the built-in layout, other keys warn; the starred form also forces itemsep=0pt |
| `\newcommand` | `{\name}[n]{body}` | defines a macro with 0-9 arguments; rejects an existing name |
| `\renewcommand` | `{\name}[n]{body}` | redefines an existing macro |
| `\DeclareMathOperator` | `*{\name}{text}` | defines \name as \operatorname{text}; the starred form takes limits |
| `\input` | `{path}` | expands a project-relative document in place |
| `\include` | `{path}` | expands a project-relative document in place |
| `\verbatiminput` | `*{file}` | verbatim: the project file's raw bytes as literal monospaced lines, exactly like a verbatim body holding those bytes; the starred form marks spaces (needs verbatim) |
| `\lstinputlisting` | `[options]{file}` | listings: the project file's raw bytes as literal monospaced lines, exactly like an lstlisting body holding those bytes; the options are read and ignored (needs listings) |
| `\label` | `{key}` | names the current section, equation or figure number |
| `\ref` | `{key}` | number of the labelled item |
| `\pageref` | `{key}` | page number of the labelled item, in the \pagenumbering style in force at the label |
| `\thepage` |  | current page's number, resolved when the page is set, in the \pagenumbering style in force here |
| `\eqref` | `{key}` | parenthesised equation number of the labelled item |
| `\cref` | `*{key list}` | cleveref lower-case named references; consecutive ranges are compressed |
| `\Cref` | `*{key list}` | cleveref capitalised named references; consecutive ranges are compressed |
| `\crefrange` | `*{first}{last}` | cleveref named reference range |
| `\Crefrange` | `*{first}{last}` | capitalised cleveref named reference range |
| `\cpageref` | `*{key list}` | cleveref named page references |
| `\Cpageref` | `*{key list}` | capitalised cleveref named page references |
| `\labelcref` | `*{key list}` | cleveref label text without the reference name |
| `\crefname` | `{type}{singular}{plural}` | cleveref lower-case singular and plural name override |
| `\Crefname` | `{type}{singular}{plural}` | cleveref capitalised singular and plural name override |
| `\numberwithin` | `[\style]{counter}{parent}` | amsmath: counter reset by parent and printed \theparent.\style{counter} (equation, figure, table; theorem counters within section) |
| `\counterwithin` | `{counter}{parent}` | counter reset by parent and printed \theparent.\arabic{counter}; starred form keeps the printed form |
| `\counterwithout` | `{counter}{parent}` | undoes \counterwithin; starred form keeps the printed form |
| `\caption` | `{...}` | numbered "Figure N:" caption inside figure |
| `\captionof` | `{type}[short]{...}` | numbered caption outside a float: "Figure N:" for figure, "Table N:" for table |
| `\item` | `[label]` | entry of an itemize, enumerate or description list |
| `\includegraphics` | `*[keys]{file}` | image box in running text (graphicx keys as written) |
| `\scalebox` | `{x}[y]{...}` | graphics.sty scaled box of the content |
| `\resizebox` | `*{width}{height}{...}` | graphics.sty box scaled to a width and/or height; ! keeps the aspect ratio |
| `\rotatebox` | `[keys]{angle}{...}` | graphicx rotated box; the box is the rotated bounding box |
| `\reflectbox` | `{...}` | graphics.sty box mirrored left to right |
| `\graphicspath` | `{{dir/}...}` | image search directories; no material |
| `\hypersetup` | `{key=value,...}` | hyperref options; PDF annotations, outline and metadata only, so nothing is typeset for them |
| `\lstset` | `{key=value,...}` | listings defaults, global from that point on; the key names are checked and nothing is typeset here |
| `\allowdisplaybreaks` | `[0-4]` | amsmath page-break permission inside displays; no material |
| `\url` | `{url}` | monospaced URL text, breaking as url.sty does; links are not clickable |
| `\href` | `{url}{text}` | link text; links are not clickable |
| `\nolinkurl` | `{url}` | monospaced URL text without a link, breaking as url.sty does |
| `\hfill` |  | infinite-stretch horizontal glue |
| `\hrulefill` |  | \hfill filled with a 0.4pt baseline rule (latex.ltx \leaders\hrule\hfill) |
| `\dotfill` |  | \hfill filled with dots in 0.44em boxes, centred (latex.ltx \cleaders) |
| `\hfil` |  | infinite-stretch horizontal glue (same order as \hfill) |
| `\qedhere` |  | amsthm end-of-proof box on this line, flush right; the automatic box at \end{proof} is suppressed |
| `\hspace` | `{dimension}` | fixed horizontal space; starred form identical |
| `\ensuremath` | `{math}` | the argument as inline math (latex.ltx: `$...$` when not already in math mode) |
| `\hskip` | `<glue>` | TeX horizontal glue without braces: a dimension with optional plus/minus stretch and shrink, including fil/fill/filll (the expansion pass scans the glue with TeX's grammar: registers, \p@, \@plus, 0.5\textwidth, em of the current font) |
| `\vskip` | `<glue>` | TeX vertical glue without braces (scanned like \hskip): ends the paragraph and adds the glue; an infinite stretch fills the page like \vfil |
| `\kern` | `<dimen>` | TeX kern (scanned like \hskip): a fixed horizontal space in a paragraph, vertical space between paragraphs |
| `\hss` |  | infinite-stretch horizontal glue (0pt plus 1fil minus 1fil), as \hfil |
| `\vfil` |  | vertical glue filling the rest of the page (same order as \vfill) |
| `\vss` |  | vertical glue filling the rest of the page (0pt plus 1fil minus 1fil), as \vfil |
| `\pdfgentounicode` |  | pdfTeX glyph-to-Unicode switch: accepted no-op, copy-paste metadata with no visible output |
| `\pdfglyphtounicode` | `{name}{hex}` | pdfTeX glyph-to-Unicode mapping: accepted no-op, copy-paste metadata with no visible output |
| `\strut` |  | zero-width strut box, 0.7/0.3 of the current baselineskip (latex.ltx \strutbox) |
| `\footnote` | `[n]{...}` | numbered mark and page-bottom footnote text |
| `\footnotemark` | `[n]` | footnote mark only |
| `\footnotetext` | `[n]{...}` | footnote text without a mark |
| `\fnsymbol` | `{counter}` | a counter's value 1-9 as a footnote symbol |
| `\marginpar` | `[left]{right}` | margin note set in the right margin at footnotesize; always the right side, with no collision avoidance between close notes |
| `\normalfont` |  | resets the text face |
| `\bfseries` |  | switches to bold |
| `\mdseries` |  | switches to medium weight |
| `\itshape` |  | switches to italic |
| `\slshape` |  | switches to slanted (typeset as italic) |
| `\scshape` |  | switches to small capitals (upright in the Core 14 layout) |
| `\upshape` |  | switches to upright |
| `\ttfamily` |  | switches to typewriter |
| `\rmfamily` |  | switches to roman |
| `\sffamily` |  | switches to sans-serif |
| `\em` |  | toggles emphasis |
| `\bf` |  | LaTeX 2.09 form: bold roman |
| `\it` |  | LaTeX 2.09 form: italic roman |
| `\sl` |  | LaTeX 2.09 form: slanted roman |
| `\sc` |  | LaTeX 2.09 form: small-caps roman (upright in the Core 14 layout) |
| `\tt` |  | LaTeX 2.09 form: upright typewriter |
| `\rm` |  | LaTeX 2.09 form: upright roman |
| `\sf` |  | LaTeX 2.09 form: upright sans-serif |
| `\quad` |  | 1em of horizontal space |
| `\qquad` |  | 2em of horizontal space |
| `\bigskip` |  | ends the paragraph and adds 12pt of vertical space |
| `\medskip` |  | ends the paragraph and adds 6pt of vertical space |
| `\smallskip` |  | ends the paragraph and adds 3pt of vertical space |
| `\vspace` | `{dimension}` | ends the paragraph and adds fixed vertical space |
| `\hrule` |  | full-measure horizontal rule |
| `\newpage` |  | forces a page break |
| `\clearpage` |  | forces a page break |
| `\cleardoublepage` |  | forces a page break (one-sided article) |
| `\twocolumn` | `[material]` | starts a new two-column page (\clearpage, then \if@twocolumn); \textwidth, \parindent and the list margins keep the one-column class values, as in LaTeX. The optional full-width material above the columns is not implemented |
| `\onecolumn` |  | starts a new one-column page (\clearpage, then \if@twocolumn false); \columnwidth becomes \textwidth |
| `\pagebreak` | `[n]` | page-break penalty -\@getpen{n} (4: a forced break); in a paragraph, after the line it is set on |
| `\nopagebreak` | `[n]` | page-break penalty \@getpen{n}; in a paragraph, after the line it is set on |
| `\linebreak` | `[n]` | line-break penalty -\@getpen{n} (4: a forced break, the line stays justified) |
| `\nolinebreak` | `[n]` | line-break penalty \@getpen{n}, the space before it moved after it |
| `\penalty` | `<number>` | penalty node: in a paragraph a line-break penalty, between paragraphs a page-break penalty |
| `\nobreak` |  | \penalty10000 |
| `\allowbreak` |  | \penalty0 |
| `\goodbreak` |  | ends the paragraph, then \penalty-500 |
| `\filbreak` |  | ends the paragraph, then \vfil\penalty-200\vfilneg |
| `\discretionary` | `{pre}{post}{nobreak}` | discretionary break (plain text of each argument) |
| `\nobreakdash` | `- -- ---` | amsmath: the dashes that follow, with no line break after them (\nobreak) |
| `\tolerance` | `=<number>` | line-breaking parameter, restored at the end of its group |
| `\pretolerance` | `=<number>` | line-breaking parameter, restored at the end of its group |
| `\looseness` | `=<number>` | line-breaking parameter for the next paragraph end |
| `\widowpenalty` | `=<number>` | page-breaking parameter, restored at the end of its group |
| `\clubpenalty` | `=<number>` | page-breaking parameter, restored at the end of its group |
| `\interlinepenalty` | `=<number>` | page-breaking parameter, restored at the end of its group |
| `\emergencystretch` | `=<dimen>` | line-breaking parameter, restored at the end of its group |
| `\sloppy` |  | \tolerance 9999, \emergencystretch 3em, \hfuzz .5pt |
| `\fussy` |  | \tolerance 200, \emergencystretch 0pt, \hfuzz .1pt |
| `\samepage` |  | \interlinepenalty 10000 for the rest of the group |
| `\raggedbottom` |  | pages keep their natural height |
| `\flushbottom` |  | pages are stretched to the text height |
| `\enlargethispage` | `*{dimension}` | the current page's text height grows by the dimension (* also shrinks its glue); pt/cm/.../\baselineskip multiples |
| `\hyphenation` | `{words}` | hyphenation exceptions: the hyphens mark each word's only break points |
| `\vfill` |  | vertical glue filling the rest of the page |
| `\columnbreak` | `[n]` | multicol: ends the current column of multicols (priority n, default 4) |
| `\newcolumn` |  | multicol: ends the current column of multicols, filling it |
| `\raggedcolumns` |  | multicol: columns keep their natural height |
| `\flushcolumns` |  | multicol: columns are stretched to one height (the default) |
| `\pagestyle` | `{style}` | records a page-style switch per page: fancy ships the fancyhead/fancyfoot fields, every other style renders no headers or footers |
| `\thispagestyle` | `{style}` | records a one-page style switch: fancy ships the fancyhead/fancyfoot fields, every other style renders no headers or footers |
| `\fancyhead` | `[pos]{...}` | fancyhdr: sets the header fields for positions L, C, R (combinable with E/O, as in [LE,RO]); empty content clears them |
| `\fancyfoot` | `[pos]{...}` | fancyhdr: sets the footer fields for positions L, C, R (combinable with E/O, as in [LE,RO]); empty content clears them |
| `\fancyhf` | `[pos]{...}` | fancyhdr: sets all six header and footer fields at once; empty content clears them |
| `\lhead` | `[even]{...}` | fancyhdr: sets the left header field (the optional even-page group is consumed and ignored one-sided); empty content clears it |
| `\chead` | `[even]{...}` | fancyhdr: sets the centre header field (the optional even-page group is consumed and ignored one-sided); empty content clears it |
| `\rhead` | `[even]{...}` | fancyhdr: sets the right header field (the optional even-page group is consumed and ignored one-sided); empty content clears it |
| `\lfoot` | `[even]{...}` | fancyhdr: sets the left footer field (the optional even-page group is consumed and ignored one-sided); empty content clears it |
| `\cfoot` | `[even]{...}` | fancyhdr: sets the centre footer field (the optional even-page group is consumed and ignored one-sided); empty content clears it |
| `\rfoot` | `[even]{...}` | fancyhdr: sets the right footer field (the optional even-page group is consumed and ignored one-sided); empty content clears it |
| `\fancypagestyle` | `{style}{...}` | fancyhdr: recognised but not implemented (a later slice owns it) |
| `\pagenumbering` | `{style}` | resets the page counter to 1 and selects the \thepage/\pageref style (arabic, roman, Roman, alph, Alph); unknown styles fall back to arabic |
| `\listfiles` |  | accepted no-op; there is no log stream |
| `\centering` |  | centres the following paragraphs |
| `\Centering` |  | centres the following paragraphs (ragged2e form) |
| `\raggedright` |  | left-aligned following paragraphs |
| `\RaggedRight` |  | left-aligned following paragraphs (ragged2e form) |
| `\raggedleft` |  | right-aligned following paragraphs |
| `\RaggedLeft` |  | right-aligned following paragraphs (ragged2e form) |
| `\obeylines` |  | every source newline ends the line, like \\, for the rest of the group |
| `\noindent` |  | accepted no-op; paragraphs are never indented |
| `\indent` |  | accepted; the first-line indent is diagnosed, not drawn |
| `\tiny` |  | size declaration from the class size table |
| `\scriptsize` |  | size declaration from the class size table |
| `\footnotesize` |  | size declaration from the class size table |
| `\small` |  | size declaration from the class size table |
| `\normalsize` |  | size declaration: the active body size |
| `\large` |  | size declaration from the class size table |
| `\Large` |  | size declaration from the class size table |
| `\LARGE` |  | size declaration from the class size table |
| `\huge` |  | size declaration from the class size table |
| `\Huge` |  | size declaration from the class size table |
| `\fontsize` | `{size}{skip}` | NFSS: the size and baselineskip the next \selectfont selects, exactly as given (resolved by the expansion engine) |
| `\selectfont` |  | NFSS: applies the last \fontsize (its family, series and shape commands are not implemented) |
| `\larger` | `{...}` | relsize: one step up the class size table from the size in effect; without an argument, a declaration for the rest of the scope |
| `\smaller` | `{...}` | relsize: one step down the class size table from the size in effect; without an argument, a declaration for the rest of the scope |
| `\cite` | `[note]{keys}` | numbered citation from thebibliography entries, or biblatex's numeric citation when biblatex is loaded; natbib redefines it as \citet, or as \citep when an optional argument follows; with cite.sty loaded the keys are sorted, three or more consecutive numbers become a range, and the separator is cite's thin glue |
| `\parencite` | `[pre][post]{keys}` | biblatex parenthetical citation: [n] in numeric style |
| `\textcite` | `[pre][post]{keys}` | biblatex textual citation: Author [n] in numeric style |
| `\autocite` | `[pre][post]{keys}` | biblatex automatic citation, equivalent to \parencite in this compiler |
| `\citet` | `[pre][post]{keys}` | natbib textual citation: Name (Year); one optional argument is the post-note |
| `\citep` | `[pre][post]{keys}` | natbib parenthetical citation: (Name, Year); one optional argument is the post-note |
| `\citealt` | `[pre][post]{keys}` | natbib \citet without the parentheses: Name Year |
| `\citealp` | `[pre][post]{keys}` | natbib \citep without the parentheses: Name, Year |
| `\citeauthor` | `[pre][post]{keys}` | natbib author list alone, or biblatex author text; the starred form is the long list |
| `\citefullauthor` | `[pre][post]{keys}` | natbib \citeauthor*: the long author list |
| `\citeyear` | `[pre][post]{keys}` | natbib year alone, or biblatex year text |
| `\citeyearpar` | `[pre][post]{keys}` | natbib year in parentheses |
| `\citenum` | `[pre][post]{keys}` | natbib \bibitem number alone, whatever the citation style |
| `\citetext` | `{text}` | natbib's citation delimiters around arbitrary text |
| `\Citet` | `[pre][post]{keys}` | natbib \citet with the author list's first letter uppercased |
| `\Citep` | `[pre][post]{keys}` | natbib \citep with the author list's first letter uppercased |
| `\Citealt` | `[pre][post]{keys}` | natbib \citealt with the author list's first letter uppercased |
| `\Citealp` | `[pre][post]{keys}` | natbib \citealp with the author list's first letter uppercased |
| `\Citeauthor` | `[pre][post]{keys}` | natbib \citeauthor with the author list's first letter uppercased |
| `\nocite` | `{keys}` | biblatex includes keys, including * for every resource entry, without visible citation output |
| `\addbibresource` | `[location]{file}` | biblatex registers a project-relative .bib resource |
| `\printbibliography` | `[key=value,...]` | biblatex heading and formatted entries from the registered .bib resources |
| `\bibitem` | `[label]{key}` | entry of thebibliography; natbib's [Author(Year)] and [Author, Year] labels feed author-year citations |
| `\bibliography` | `{files}` | diagnosed: .bib input is not read |
| `\bibliographystyle` | `{style}` | diagnosed: no effect without .bib support |
| `\title` | `{...}` | title for \maketitle (beamer: and \titlepage, with an optional [short] form read past) |
| `\author` | `{...}` | author block for \maketitle; \and and \thanks inside it (beamer: optional [short] form read past) |
| `\date` | `{...}` | date for \maketitle; \today inside it (beamer: optional [short] form read past) |
| `\maketitle` |  | article.cls title block |
| `\address` | `{lines}` | letter.cls return address (\\-separated lines), set by \opening |
| `\signature` | `{name}` | letter.cls name under the closing; falls back to \name |
| `\name` | `{name}` | letter.cls \fromname, used when \signature is empty |
| `\location` | `{text}` | letter.cls \fromlocation: recorded; only the firstpage footer would set it |
| `\telephone` | `{number}` | letter.cls \telephonenum: recorded; only the firstpage footer would set it |
| `\opening` | `{salutation}` | letter.cls: return address and date flush right, the recipient, then the salutation |
| `\closing` | `{text}` | letter.cls: closing and signature at \longindentation, 6\parskip apart |
| `\cc` | `{text}` | letter.cls carbon-copy line, labelled 'cc:' |
| `\encl` | `{text}` | letter.cls enclosure line, labelled 'encl:' |
| `\hangfrom` | `{label}` | kernel (ltsect.dtx): label set inline, continuing the paragraph; the hanging indent itself is not applied |
| `\ps` |  | letter.cls postscript: a paragraph break and nothing else — it takes no argument |
| `\startbreaks` |  | letter.cls: re-allows page breaks after \closing; no effect on this layout |
| `\stopbreaks` |  | letter.cls: forbids page breaks inside the closing; no effect on this layout |
| `\stopletter` |  | letter.cls hook run at \end{letter}; empty in the class itself |
| `\makelabels` |  | letter.cls address-label page: accepted, not produced (no .aux round trip) |
| `\today` |  | the date carried by the compile request; this compiler never reads the clock |
| `\TeX` |  | latex.ltx logo: T, kern -.1667em, E lowered .5ex, kern -.125em, X |
| `\LaTeX` |  | latex.ltx logo: L, kern -.36em, script-size A raised to the T height, kern -.15em, \TeX |
| `\LaTeXe` |  | \LaTeX, kern .15em, 2 and a text-style subscript varepsilon |
| `\rule` | `[raise]{dimension}{dimension}` | filled rule box; pt/in/cm/mm/bp/dd/cc/pc/sp, em, ex, \textwidth, \linewidth, \columnwidth |
| `\mbox` | `{...}` | kernel unbreakable box: the argument as one \hbox at its natural width, never broken across lines (also in math) |
| `\phantom` | `{...}` | kernel invisible box: the argument's full width, height and depth, paints nothing (single-line; also in math) |
| `\hphantom` | `{...}` | kernel invisible box: the argument's width only, zero height and depth (single-line; also in math) |
| `\vphantom` | `{...}` | kernel invisible box: the argument's height and depth only, zero width (single-line; also in math) |
| `\thinspace` |  | text kern .16667em (math: thin muskip) |
| `\negthinspace` |  | text kern -.16667em |
| `\medspace` |  | text kern .2222em |
| `\negmedspace` |  | text kern -.2222em |
| `\thickspace` |  | text kern .2777em |
| `\negthickspace` |  | text kern -.2777em |
| `\enspace` |  | text kern .5em |
| `\enskip` |  | horizontal glue of .5em |
| `\xspace` |  | word space unless the next token is }, , . ' / ? ; : ! ~ - ), or a short suppressing-command list (\footnote, \footnotemark, \bgroup, \egroup, control space) |
| `\CJKfamily` | `{family}` | CJKutf8: selects the CJK family (min, goth, maru, gbsn, gkai, bsmi, bkai, mj) for the rest of the group inside a CJK environment; an unknown family sets nothing, as pdflatex's C70/song substitution does |
| `\CJKspace` |  | CJKutf8: a source blank after a CJK character is an interword space again (undoes \CJKnospace / CJK*) |
| `\CJKnospace` |  | CJKutf8: a source blank after a CJK character is ignored, as in the CJK* environment |
| `\CJKtilde` |  | CJKutf8: makes ~ a no-break space, which it already is; accepted no-op |
| `\AA` |  | text symbol \AA: OT1 Å, T1 Å (tex-text-encoding; unavailable is a LaTeX error) |
| `\aa` |  | text symbol \aa: OT1 å, T1 å (tex-text-encoding; unavailable is a LaTeX error) |
| `\AE` |  | text symbol \AE: OT1 Æ, T1 Æ (tex-text-encoding; unavailable is a LaTeX error) |
| `\ae` |  | text symbol \ae: OT1 æ, T1 æ (tex-text-encoding; unavailable is a LaTeX error) |
| `\OE` |  | text symbol \OE: OT1 Œ, T1 Œ (tex-text-encoding; unavailable is a LaTeX error) |
| `\oe` |  | text symbol \oe: OT1 œ, T1 œ (tex-text-encoding; unavailable is a LaTeX error) |
| `\O` |  | text symbol \O: OT1 Ø, T1 Ø (tex-text-encoding; unavailable is a LaTeX error) |
| `\o` |  | text symbol \o: OT1 ø, T1 ø (tex-text-encoding; unavailable is a LaTeX error) |
| `\L` |  | text symbol \L: OT1 Ł, T1 Ł (tex-text-encoding; unavailable is a LaTeX error) |
| `\l` |  | text symbol \l: OT1 ł, T1 ł (tex-text-encoding; unavailable is a LaTeX error) |
| `\ss` |  | text symbol \ss: OT1 ß, T1 ß (tex-text-encoding; unavailable is a LaTeX error) |
| `\SS` |  | text symbol \SS: OT1 SS, T1 ẞ (tex-text-encoding; unavailable is a LaTeX error) |
| `\TH` |  | text symbol \TH: OT1 unavailable, T1 Þ (tex-text-encoding; unavailable is a LaTeX error) |
| `\th` |  | text symbol \th: OT1 unavailable, T1 þ (tex-text-encoding; unavailable is a LaTeX error) |
| `\DH` |  | text symbol \DH: OT1 unavailable, T1 Ð (tex-text-encoding; unavailable is a LaTeX error) |
| `\dh` |  | text symbol \dh: OT1 unavailable, T1 ð (tex-text-encoding; unavailable is a LaTeX error) |
| `\DJ` |  | text symbol \DJ: OT1 unavailable, T1 Đ (tex-text-encoding; unavailable is a LaTeX error) |
| `\dj` |  | text symbol \dj: OT1 unavailable, T1 đ (tex-text-encoding; unavailable is a LaTeX error) |
| `\NG` |  | text symbol \NG: OT1 unavailable, T1 Ŋ (tex-text-encoding; unavailable is a LaTeX error) |
| `\ng` |  | text symbol \ng: OT1 unavailable, T1 ŋ (tex-text-encoding; unavailable is a LaTeX error) |
| `\IJ` |  | text symbol \IJ: OT1 Ĳ, T1 Ĳ (tex-text-encoding; unavailable is a LaTeX error) |
| `\ij` |  | text symbol \ij: OT1 ĳ, T1 ĳ (tex-text-encoding; unavailable is a LaTeX error) |
| `\i` |  | text symbol \i: OT1 ı, T1 ı (tex-text-encoding; unavailable is a LaTeX error) |
| `\j` |  | text symbol \j: OT1 ȷ, T1 ȷ (tex-text-encoding; unavailable is a LaTeX error) |
| `\S` |  | text symbol \textsection: OT1 §, T1 § (tex-text-encoding; unavailable is a LaTeX error) |
| `\P` |  | text symbol \textparagraph: OT1 ¶, T1 ¶ (tex-text-encoding; unavailable is a LaTeX error) |
| `\dag` |  | text symbol \textdagger: OT1 †, T1 † (tex-text-encoding; unavailable is a LaTeX error) |
| `\ddag` |  | text symbol \textdaggerdbl: OT1 ‡, T1 ‡ (tex-text-encoding; unavailable is a LaTeX error) |
| `\copyright` |  | text symbol \textcopyright: OT1 ©, T1 © (tex-text-encoding; unavailable is a LaTeX error) |
| `\pounds` |  | text symbol \textsterling: OT1 £, T1 £ (tex-text-encoding; unavailable is a LaTeX error) |
| `\dots` |  | text symbol \textellipsis: OT1 …, T1 … (tex-text-encoding; unavailable is a LaTeX error) |
| `\ldots` |  | text symbol \textellipsis: OT1 …, T1 … (tex-text-encoding; unavailable is a LaTeX error) |
| `\textsection` |  | text symbol \textsection: OT1 §, T1 § (tex-text-encoding; unavailable is a LaTeX error) |
| `\textparagraph` |  | text symbol \textparagraph: OT1 ¶, T1 ¶ (tex-text-encoding; unavailable is a LaTeX error) |
| `\textdagger` |  | text symbol \textdagger: OT1 †, T1 † (tex-text-encoding; unavailable is a LaTeX error) |
| `\textdaggerdbl` |  | text symbol \textdaggerdbl: OT1 ‡, T1 ‡ (tex-text-encoding; unavailable is a LaTeX error) |
| `\textcopyright` |  | text symbol \textcopyright: OT1 ©, T1 © (tex-text-encoding; unavailable is a LaTeX error) |
| `\textsterling` |  | text symbol \textsterling: OT1 £, T1 £ (tex-text-encoding; unavailable is a LaTeX error) |
| `\textellipsis` |  | text symbol \textellipsis: OT1 …, T1 … (tex-text-encoding; unavailable is a LaTeX error) |
| `\textbackslash` |  | text symbol \textbackslash: OT1 \, T1 \ (tex-text-encoding; unavailable is a LaTeX error) |
| `\textasciitilde` |  | text symbol \textasciitilde: OT1 ~, T1 ~ (tex-text-encoding; unavailable is a LaTeX error) |
| `\textasciicircum` |  | text symbol \textasciicircum: OT1 ^, T1 ^ (tex-text-encoding; unavailable is a LaTeX error) |
| `\textunderscore` |  | text symbol \textunderscore: OT1 _, T1 _ (tex-text-encoding; unavailable is a LaTeX error) |
| `\textbar` |  | text symbol \textbar: OT1 \|, T1 \| (tex-text-encoding; unavailable is a LaTeX error) |
| `\textless` |  | text symbol \textless: OT1 <, T1 < (tex-text-encoding; unavailable is a LaTeX error) |
| `\textgreater` |  | text symbol \textgreater: OT1 >, T1 > (tex-text-encoding; unavailable is a LaTeX error) |
| `\textbraceleft` |  | text symbol \textbraceleft: OT1 {, T1 { (tex-text-encoding; unavailable is a LaTeX error) |
| `\textbraceright` |  | text symbol \textbraceright: OT1 }, T1 } (tex-text-encoding; unavailable is a LaTeX error) |
| `\textbullet` |  | text symbol \textbullet: OT1 •, T1 • (tex-text-encoding; unavailable is a LaTeX error) |
| `\textperiodcentered` |  | text symbol \textperiodcentered: OT1 ·, T1 · (tex-text-encoding; unavailable is a LaTeX error) |
| `\textregistered` |  | text symbol \textregistered: OT1 ®, T1 ® (tex-text-encoding; unavailable is a LaTeX error) |
| `\texttrademark` |  | text symbol \texttrademark: OT1 ™, T1 ™ (tex-text-encoding; unavailable is a LaTeX error) |
| `\textdegree` |  | text symbol \textdegree: OT1 °, T1 ° (tex-text-encoding; unavailable is a LaTeX error) |
| `\textmu` |  | text symbol \textmu: OT1 µ, T1 µ (tex-text-encoding; unavailable is a LaTeX error) |
| `\textohm` |  | text symbol \textohm: OT1 Ω, T1 Ω (tex-text-encoding; unavailable is a LaTeX error) |
| `\textcelsius` |  | text symbol \textcelsius: OT1 ℃, T1 ℃ (tex-text-encoding; unavailable is a LaTeX error) |
| `\texteuro` |  | text symbol \texteuro: OT1 €, T1 € (tex-text-encoding; unavailable is a LaTeX error) |
| `\textyen` |  | text symbol \textyen: OT1 ¥, T1 ¥ (tex-text-encoding; unavailable is a LaTeX error) |
| `\textwon` |  | text symbol \textwon: OT1 ₩, T1 ₩ (tex-text-encoding; unavailable is a LaTeX error) |
| `\textcurrency` |  | text symbol \textcurrency: OT1 ¤, T1 ¤ (tex-text-encoding; unavailable is a LaTeX error) |
| `\textestimated` |  | text symbol \textestimated: OT1 ℮, T1 ℮ (tex-text-encoding; unavailable is a LaTeX error) |
| `\textnumero` |  | text symbol \textnumero: OT1 №, T1 № (tex-text-encoding; unavailable is a LaTeX error) |
| `\textrecipe` |  | text symbol \textrecipe: OT1 ℞, T1 ℞ (tex-text-encoding; unavailable is a LaTeX error) |
| `\textservicemark` |  | text symbol \textservicemark: OT1 ℠, T1 ℠ (tex-text-encoding; unavailable is a LaTeX error) |
| `\textbardbl` |  | text symbol \textbardbl: OT1 ‖, T1 ‖ (tex-text-encoding; unavailable is a LaTeX error) |
| `\textbrokenbar` |  | text symbol \textbrokenbar: OT1 ¦, T1 ¦ (tex-text-encoding; unavailable is a LaTeX error) |
| `\texttimes` |  | text symbol \texttimes: OT1 ×, T1 × (tex-text-encoding; unavailable is a LaTeX error) |
| `\textdiv` |  | text symbol \textdiv: OT1 ÷, T1 ÷ (tex-text-encoding; unavailable is a LaTeX error) |
| `\textonehalf` |  | text symbol \textonehalf: OT1 ½, T1 ½ (tex-text-encoding; unavailable is a LaTeX error) |
| `\textonequarter` |  | text symbol \textonequarter: OT1 ¼, T1 ¼ (tex-text-encoding; unavailable is a LaTeX error) |
| `\textthreequarters` |  | text symbol \textthreequarters: OT1 ¾, T1 ¾ (tex-text-encoding; unavailable is a LaTeX error) |
| `\textperthousand` |  | text symbol \textperthousand: OT1 ‰, T1 ‰ (tex-text-encoding; unavailable is a LaTeX error) |
| `\textpertenthousand` |  | text symbol \textpertenthousand: OT1 ‱, T1 ‱ (tex-text-encoding; unavailable is a LaTeX error) |
| `\textopenbullet` |  | text symbol \textopenbullet: OT1 ◦, T1 ◦ (tex-text-encoding; unavailable is a LaTeX error) |
| `\textlangle` |  | text symbol \textlangle: OT1 〈, T1 〈 (tex-text-encoding; unavailable is a LaTeX error) |
| `\textrangle` |  | text symbol \textrangle: OT1 〉, T1 〉 (tex-text-encoding; unavailable is a LaTeX error) |
| `\c` | `{letter}` | cedilla text accent: the precomposed character the dfu tables declare (tex-text-encoding); without one the bare letter and a warning |
| `\v` | `{letter}` | caron text accent: the precomposed character the dfu tables declare (tex-text-encoding); without one the bare letter and a warning |
| `\u` | `{letter}` | breve text accent: the precomposed character the dfu tables declare (tex-text-encoding); without one the bare letter and a warning |
| `\H` | `{letter}` | double acute text accent: the precomposed character the dfu tables declare (tex-text-encoding); without one the bare letter and a warning |
| `\r` | `{letter}` | ring text accent: the precomposed character the dfu tables declare (tex-text-encoding); without one the bare letter and a warning |
| `\k` | `{letter}` | ogonek text accent (T1 only; OT1 reports it unavailable): the precomposed character the dfu tables declare; without one the bare letter and a warning |
| `\d` | `{letter}` | dot-below text accent: the precomposed character the dfu tables declare (tex-text-encoding); without one the bare letter and a warning |
| `\b` | `{letter}` | bar-below text accent: no dfu declarations, so the bare letter and a warning |
| `\capitalcaron` | `{letter}` | capital caron text accent: an alias of \v, the precomposed character the dfu tables declare |
| `\capitalbreve` | `{letter}` | capital breve text accent: an alias of \u, the precomposed character the dfu tables declare |
| `\capitalring` | `{letter}` | capital ring text accent: an alias of \r, the precomposed character the dfu tables declare |
| `\capitalogonek` | `{letter}` | capital ogonek text accent: an alias of \k (T1 only; OT1 reports it unavailable) |
| `\capitalhungarumlaut` | `{letter}` | capital double-acute text accent: an alias of \H, the precomposed character the dfu tables declare |
| `\capitalcedilla` | `{letter}` | capital cedilla text accent: an alias of \c, the precomposed character the dfu tables declare |
| `\uline` | `{...}` | ulem underline: 0.4pt rule under the argument (single-line; needs ulem) |
| `\underline` | `{...}` | kernel text underline: TeXbook Rule 10 math-rule under an unbreakable hbox |
| `\underbar` | `{...}` | kernel text underline: Rule 10 rule like \underline but content depth zeroed (fixed position) |
| `\sout` | `{...}` | ulem strike-out: 0.4pt rule 0.55ex above the baseline (single-line; needs ulem) |
| `\textsuperscript` | `{...}` | kernel text superscript: argument at \sf@size raised like a math superscript (single-line) |
| `\textsubscript` | `{...}` | kernel text subscript: argument at \sf@size lowered like a math subscript (single-line) |
| `\newtheorem` | `{env}[counter]{name}` | defines a numbered theorem-like environment (amsthm) |
| `\theoremstyle` | `{style}` | selects the amsthm style for following \newtheorem |
| `\so` | `{...}` | soul letterspacing: 0.25em kern between the argument's letters, 0.65em word spaces (0.55em at the edges) (single-line; needs soul) |
| `\hl` | `{...}` | soul highlight: yellow behind-text rule at the argument's natural width, 1.75ex above and 0.75ex below the baseline (single-line; interword gaps between fragments are not painted, see GH-828; needs soul) |
| `\text` | `{...}` | amsmath text in text mode: outside math simply \mbox, the argument as one unbreakable box in the current style |
| `\boxed` | `{...}` | amsmath box in text mode: the argument with a drawn frame (\fbox with math inside) |
| `\enquote` | `{text}` | csquotes: wraps text in typographic quotation marks; nesting alternates double \u{201c}\u{201d} and single \u{2018}\u{2019} (needs csquotes) |
| `\frametitle` | `{...}` | beamer frame title (\Large, structure colour, in the frametitle box at the top of the slide); optional <overlay> and [short] read past; needs \documentclass{beamer} |
| `\framesubtitle` | `{...}` | beamer frame subtitle (\footnotesize, under the frame title); needs \documentclass{beamer} |
| `\alert` | `<overlay>{...}` | beamer alert text in red on the slides the <overlay> spec selects (every slide without one); needs \documentclass{beamer} |
| `\pause` | `[n]` | beamer: the material after it is covered (space kept, not painted) until slide n, the pause count + 1 by default; needs \documentclass{beamer} |
| `\onslide` | `<overlay>{...}` | beamer: without an argument, the material up to the next \onslide or \pause is covered on the slides the spec does not select; with one, like \uncover (\onslide* like \only, \onslide+ like \visible); needs \documentclass{beamer} |
| `\uncover` | `<overlay>{...}` | beamer: the argument (text, formulas, tabular, includegraphics) keeps its space and is not painted on the slides the spec does not select (\setbeamercovered{invisible}), or painted mixed with the background under \setbeamercovered{transparent}; needs \documentclass{beamer} |
| `\only` | `<overlay>{...}` | beamer: the argument is typeset only on the slides the spec selects and takes no space on the others; needs \documentclass{beamer} |
| `\visible` | `<overlay>{...}` | beamer: like \uncover; needs \documentclass{beamer} |
| `\invisible` | `<overlay>{...}` | beamer: the argument is covered on the slides the spec selects; needs \documentclass{beamer} |
| `\temporal` | `<overlay>{before}{during}{after}` | beamer: {during} on the slides the spec selects, {before} on earlier slides, {after} on later ones, each taking space only where it shows; needs \documentclass{beamer} |
| `\subtitle` | `{...}` | beamer subtitle for \titlepage; optional [short] read past; needs \documentclass{beamer} |
| `\institute` | `{...}` | beamer institute for \titlepage; optional [short] read past; needs \documentclass{beamer} |
| `\logo` | `{...}` | beamer logo, set on every frame right-aligned above the navigation symbols (the sidebar right template); needs \documentclass{beamer} |
| `\AtBeginSection` | `[special]{code}` | beamer: code run at every \section (the [special] code at a starred one), e.g. an outline frame; needs \documentclass{beamer} |
| `\AtBeginSubsection` | `[special]{code}` | beamer: code run at every \subsection (the [special] code at a starred one); needs \documentclass{beamer} |
| `\AtBeginSubsubsection` | `[special]{code}` | beamer: code run at every \subsubsection (the [special] code at a starred one); needs \documentclass{beamer} |
| `\titlepage` |  | beamer title page (default template: centred title, subtitle, author, institute, date); needs \documentclass{beamer} |
| `\note` | `{...}` | beamer note: typesets nothing (notes are shown only with \setbeameroption{show notes}); needs \documentclass{beamer} |
| `\frame` | `<overlay>[options]{...}` | beamer frame as a command (the body brace group is the slide, like \begin{frame}...\end{frame}); needs \documentclass{beamer} |
| `\usetheme` | `{...}` | beamer theme: Madrid (infolines outer, rounded inner, whale/orchid colours) is modelled, any other name keeps the default theme; needs \documentclass{beamer} |
| `\usecolortheme` | `{...}` | accepted and read past: only beamer's default colour theme is modelled; needs \documentclass{beamer} |
| `\usefonttheme` | `{...}` | accepted and read past: only beamer's default font theme is modelled; needs \documentclass{beamer} |
| `\useinnertheme` | `{...}` | accepted and read past: only beamer's default inner theme is modelled; needs \documentclass{beamer} |
| `\useoutertheme` | `{...}` | accepted and read past: only beamer's default outer theme is modelled; needs \documentclass{beamer} |
| `\setbeamertemplate` | `{...}{...}` | accepted and read past; \setbeamertemplate{navigation symbols}{} removes the navigation symbol strip the renderer draws on every non-plain frame page, any other replacement template keeps the default strip; needs \documentclass{beamer} |
| `\setbeamercolor` | `{...}{...}` | accepted and read past: beamer's default colours stay in force; needs \documentclass{beamer} |
| `\setbeamerfont` | `{...}{...}` | accepted and read past: beamer's default fonts stay in force; needs \documentclass{beamer} |
| `\setbeamercovered` | `{...}` | beamer: read by the renderer from the preamble: invisible (the default) paints nothing for covered material, transparent[=pct] paints it in its colour mixed pct% with the page background (dynamic and highly dynamic are taken as their first step, 10 and 15); needs \documentclass{beamer} |
| `\setbeamersize` | `{...}` | accepted and read past: beamer's default text margins stay in force; needs \documentclass{beamer} |
| `\beamertemplatenavigationsymbolsempty` |  | beamer: removes the navigation symbol strip the renderer draws at the bottom right of every non-plain frame page; needs \documentclass{beamer} |
| `\column` | `{width}` | beamer column inside columns: a minipage of the given width (.5\textwidth, 4cm) set beside the others; optional [c\|t\|T\|b] alignment; needs \documentclass{beamer} |
| `\titleformat` | `{\section}{format}{label}{sep}{before}[after]` | titlesec: \section headings take the format's face and size (an empty label prints no number); a \titlerule after-code draws the full-width rule; other levels are diagnosed (needs titlesec) |
| `\titlerule` |  | titlesec: a rule filling the rest of the line, or the full text width between paragraphs (needs titlesec) |
| `\hline` |  | table rule across the row, at the start of a row |
| `\cline` | `{i-j}` | partial rule over columns i to j, at the start of a row |
| `\multicolumn` | `{n}{spec}{text}` | entry spanning n columns with its own column specification |
| `\tabularnewline` |  | ends the table row |
| `\toprule` | `[width]` | booktabs rule at the top of the table (needs booktabs) |
| `\midrule` | `[width]` | booktabs rule between table rows (needs booktabs) |
| `\bottomrule` | `[width]` | booktabs rule at the bottom of the table (needs booktabs) |
| `\cmidrule` | `[width](trim){i-j}` | booktabs partial rule over columns i to j (needs booktabs) |
| `\addlinespace` | `[width]` | booktabs vertical space between rows (needs booktabs) |
| `\specialrule` | `{width}{above}{below}` | booktabs rule with explicit space around it (needs booktabs) |
| `\morecmidrules` |  | booktabs: another \cmidrule after the previous one (needs booktabs) |
| `\multirow` | `[vpos]{rows}[bigstruts]{width}[vmove]{text}` | entry spanning rows (needs multirow) |
| `\rowcolor` | `[model]{spec}` | colortbl: background colour of the next row (needs colortbl) |
| `\cellcolor` | `[model]{spec}` | colortbl: background colour of the entry (needs colortbl) |
| `\columncolor` | `[model]{spec}` | colortbl: colour of a column, in >{} (needs colortbl) |
| `\kill` |  | longtable: ends the row, which then only contributes its widths (needs longtable) |
| `\endfirsthead` |  | longtable: ends the first-page head (needs longtable) |
| `\endhead` |  | longtable: ends the repeated head (needs longtable) |
| `\endfoot` |  | longtable: ends the repeated foot (needs longtable) |
| `\endlastfoot` |  | longtable: ends the last-page foot (needs longtable) |
| `\\` |  | line break; an optional [length] is consumed |
| `\-` |  | discretionary hyphen: a break point, invisible unless the line breaks there |
| `\,` |  | text kern .16667em (\thinspace) |
| `\!` |  | text kern -.16667em (\negthinspace) |
| `\:` |  | text kern .2222em (\medspace) |
| `\>` |  | text kern .2222em (\medspace) |
| `\;` |  | text kern .2777em (\thickspace) |

### Commands run by the expansion pass

| Command | Arguments | Behaviour |
| --- | --- | --- |
| `\long` |  | prefix: the following definition accepts \par in arguments |
| `\protected` |  | e-TeX prefix: the following macro is not expanded inside \edef-like contexts |
| `\providecommand` | `{\name}[n][default]{body}` | defines the macro only when \name is undefined |
| `\DeclareRobustCommand` | `{\name}[n][default]{body}` | defines or redefines a macro (robustness is not modelled separately) |
| `\newenvironment` | `{env}[n][default]{begin}{end}` | defines an environment run by \begin{env}/\end{env} |
| `\renewenvironment` | `{env}[n][default]{begin}{end}` | redefines an environment |
| `\newcounter` | `{counter}[within]` | allocates a counter (\c@counter, \thecounter) reset by within |
| `\setcounter` | `{counter}{number}` | sets a counter globally |
| `\addtocounter` | `{counter}{number}` | adds to a counter globally |
| `\stepcounter` | `{counter}` | increments a counter and resets its dependants |
| `\refstepcounter` | `{counter}` | increments a counter and makes it the current \label value |
| `\value` | `{counter}` | a counter's value in a number context |
| `\Alph` | `{counter}` | a counter as an upper-case letter |
| `\newlength` | `{\name}` | allocates a skip register |
| `\settowidth` | `{\name}{text}` | sets a length from text measured by the expansion pass's box measurer (an approximation) |
| `\settoheight` | `{\name}{text}` | sets a length from text height (an approximation, as \settowidth) |
| `\settodepth` | `{\name}{text}` | sets a length from text depth (an approximation, as \settowidth) |
| `\AtBeginDocument` | `{code}` | stores code that runs at \begin{document} |
| `\AtEndDocument` | `{code}` | stores code that runs at \end{document} |
| `\makeatother` |  | makes @ an other character again |
| `\space` |  | expands to one space |
| `\ignorespaces` |  | skips the spaces that follow |
| `\jobname` |  | expands to texput |
| `\ifthenelse` | `{test}{true}{false}` | the ifthen package's conditional: \equal, \NOT, \AND, \OR, \isodd, \isundefined, \lengthtest and \boolean tests select one branch at expansion time |
| `\IfFileExists` | `{file}{true}{false}` | expands to the true branch if the file is present in the project closure, otherwise the false branch |
| `\arabic` | `{counter}` | a counter in arabic numerals |
| `\roman` | `{counter}` | a counter in lower-case roman numerals |
| `\Roman` | `{counter}` | a counter in upper-case roman numerals |
| `\alph` | `{counter}` | a counter as a lower-case letter |
| `\arraystretch` |  | row-stretch factor tables read at \begin{tabular} (1 by default); set with \renewcommand |
| `\newif` | `{\ifname}` | allocates a TeX conditional read with \footrue and \foofalse |
| `\verb` | `\|text\|` | literal text up to the next delimiter character |
| `\iftoggle` | `{name}{true}{false}` | the etoolbox toggle conditional: the named toggle (\newtoggle/\providetoggle declare it false, \toggletrue/\togglefalse set it) selects one branch at expansion time |
| `\RequirePackage` | `[options]{a,b}[version]` | loads project .sty files through the expansion engine (once each, with LaTeX's option clash check); built-in and missing packages reach the parser as \usepackage |
| `\RequirePackageWithOptions` | `{package}` | \RequirePackage with the current package's options |
| `\LoadClass` | `[options]{class}[version]` | in a project .cls: loads a project class file, or gives the document the standard class's page model as \documentclass[options]{class} |
| `\LoadClassWithOptions` | `{class}` | \LoadClass with the current class's options |
| `\DeclareOption` | `{option}{code}` | in a project .sty/.cls: declares an option (\DeclareOption* the handler for undeclared ones, with \CurrentOption) |
| `\CurrentOption` |  | the option being processed, in a \DeclareOption* handler |
| `\ProcessOptions` |  | runs the declared options the class and the \usepackage gave (\ProcessOptions* in the order given); an undeclared package option is LaTeX's error, an undeclared class option is ignored |
| `\ExecuteOptions` | `{a,b}` | runs declared options as defaults |
| `\OptionNotUsed` |  | in a class's \DeclareOption*: records the option as unused |
| `\PassOptionsToPackage` | `{options}{package}` | queues options for a later \usepackage of that package |
| `\PassOptionsToClass` | `{options}{class}` | queues options for a later \LoadClass of that class |
| `\AtEndOfPackage` | `{code}` | runs code when the current .sty file ends |
| `\AtEndOfClass` | `{code}` | runs code when the current .cls file ends |
| `\typeout` | `{text}` | accepted no-op; there is no terminal |
| `\wlog` | `{text}` | accepted no-op; there is no log stream |
| `\glueexpr` | `<glue expression>` | e-TeX glue expression (+, -, * and / by an integer, parentheses), also what \setlength and \addtolength evaluate their argument with |

### Math structures

| Command | Arguments | Behaviour |
| --- | --- | --- |
| `\,` |  | thin space (3mu) |
| `\:` |  | medium space (4mu) |
| `\>` |  | medium space (4mu) |
| `\;` |  | thick space (5mu) |
| `\ ` |  | control space (6mu) |
| `\!` |  | negative thin space (-3mu) |
| `\\|` |  | double vertical bar |
| `\color` | `[model]{expression}` | colours the rest of the math group |
| `\textcolor` | `[model]{expression}{body}` | math body in a colour |
| `\num` | `[options]{number}` | siunitx number or ;-separated list inside a formula |
| `\numlist` | `[options]{number}` | siunitx number or ;-separated list inside a formula |
| `\unit` | `[options]{units}` | siunitx unit inside a formula |
| `\si` | `[options]{units}` | siunitx unit inside a formula |
| `\qty` | `[options]{number}{units}` | siunitx quantity (3mu thin space before the unit) or number range inside a formula |
| `\numrange` | `[options]{number}{units}` | siunitx quantity (3mu thin space before the unit) or number range inside a formula |
| `\SI` | `[options]{number}[pre-unit]{units}` | siunitx v2 quantity inside a formula |
| `\qtylist` | `[options]{numbers}{units}` | siunitx list of quantities inside a formula |
| `\SIlist` | `[options]{numbers}{units}` | siunitx list of quantities inside a formula |
| `\qtyrange` | `[options]{number}{number}{units}` | siunitx range of quantities inside a formula |
| `\SIrange` | `[options]{number}{number}{units}` | siunitx range of quantities inside a formula |
| `\ang` | `[options]{angle}` | siunitx angle inside a formula |
| `\sisetup` | `{options}` | siunitx settings changed inside a formula |
| `\rule` | `[raise]{dimension}{dimension}` | latex.ltx \rule box in a formula, em/ex of the text font |
| `\frac` | `{num}{den}` | fraction |
| `\cfrac` | `[pos]{num}{den}` | amsmath continued fraction: display style at every level, a \strut heading each numerator, \kern-\nulldelimiterspace after; [l]/[r] warn and centre |
| `\strut` |  | latex.ltx \strutbox in a formula: an ordinary box of no width, 0.7/0.3 of the text size's baselineskip |
| `\dfrac` | `{num}{den}` | amsmath \genfrac fraction in display or text style |
| `\tfrac` | `{num}{den}` | amsmath \genfrac fraction in display or text style |
| `\genfrac` | `{left}{right}{thickness}{style}{num}{den}` | amsmath generalized fraction: delimiters, pt rule thickness and a 0-3 style |
| `\phantom` | `{x}` | empty box with the width and/or height and depth of the argument |
| `\hphantom` | `{x}` | empty box with the width and/or height and depth of the argument |
| `\vphantom` | `{x}` | empty box with the width and/or height and depth of the argument |
| `\mathllap` | `{x}` | mathtools zero-width box: the argument is painted but advances nothing, hanging left, right, or centred (\llap/\rlap/\clap); needs mathtools |
| `\mathrlap` | `{x}` | mathtools zero-width box: the argument is painted but advances nothing, hanging left, right, or centred (\llap/\rlap/\clap); needs mathtools |
| `\mathclap` | `{x}` | mathtools zero-width box: the argument is painted but advances nothing, hanging left, right, or centred (\llap/\rlap/\clap); needs mathtools |
| `\xrightarrow` | `[below]{above}` | amsmath/mathtools extensible arrow stretched to its labels (\ext@arrow) |
| `\xleftarrow` | `[below]{above}` | amsmath/mathtools extensible arrow stretched to its labels (\ext@arrow) |
| `\xleftrightarrow` | `[below]{above}` | amsmath/mathtools extensible arrow stretched to its labels (\ext@arrow) |
| `\substack` | `{a \\ b}` | amsmath centred script-style rows for limits |
| `\sqrt` | `[index]{x}` | radical with optional raised index |
| `\binom` | `{n}{k}` | amsmath binomial: zero-thickness \genfrac in parentheses; d/t forms force the style |
| `\dbinom` | `{n}{k}` | amsmath binomial: zero-thickness \genfrac in parentheses; d/t forms force the style |
| `\tbinom` | `{n}{k}` | amsmath binomial: zero-thickness \genfrac in parentheses; d/t forms force the style |
| `\choose` |  | TeX infix binomial and fraction inside a group |
| `\over` |  | TeX infix binomial and fraction inside a group |
| `\overset` | `{script}{base}` | script-size list centred above or below a base |
| `\stackrel` | `{script}{base}` | script-size list centred above or below a base |
| `\underset` | `{script}{base}` | script-size list centred above or below a base |
| `\sideset` | `{left scripts}{right scripts}{operator}` | scripts on both sides of a large operator, set in \displaystyle (\mathop) |
| `\operatorname` | `{name}` | upright named operator (\mathop); starred and withlimits forms take limits |
| `\operatornamewithlimits` | `{name}` | upright named operator (\mathop); starred and withlimits forms take limits |
| `\colon` |  | function-arrow colon: punctuation (0mu/3mu) as the kernel declares it, amsmath's 2mu/6mu when amsmath is loaded |
| `\eqqcolon` |  | mathtools =: (reverse of \coloneqq) as a relation; needs mathtools |
| `\Coloneqq` |  | mathtools ::= and =:: (each three real glyphs) as one relation; needs mathtools |
| `\Eqqcolon` |  | mathtools ::= and =:: (each three real glyphs) as one relation; needs mathtools |
| `\vcentcolon` |  | mathtools vertically centred colon: the same glyph as \colon as a relation; needs mathtools |
| `\dblcolon` |  | mathtools double vertically centred colon (two \vcentcolon) as one relation; needs mathtools |
| `\bmod` |  | upright mod |
| `\mod` |  | upright mod |
| `\pmod` | `{n}` | parenthesised (mod n) |
| `\pod` | `{n}` | amsmath parenthesised (n): like \pmod without the mod text; needs amsmath |
| `\allowbreak` |  | zero-penalty breakpoint in a formula (\penalty0); layout-neutral, formulas never break |
| `\mathbb` | `{A-Z}` | double-struck capitals from Latin Modern Math; other arguments are diagnosed |
| `\mathfrak` | `{letters}` | Euler Fraktur letters as Unicode mathematical fraktur; digits and other characters unchanged |
| `\mathcal` | `{A-Z}` | script capitals from New Computer Modern Math at cmsy10 metrics; other arguments are diagnosed |
| `\varnothing` |  | empty set at msbm10's 0.7778em advance (\emptyset's glyph) |
| `\Diamond` |  | amsfonts alias of \lozenge (msam, 0.6667em); requires amsfonts/amssymb |
| `\iff` |  | long double arrow between thick (5mu) spaces |
| `\implies` |  | long double arrow between thick (5mu) spaces |
| `\impliedby` |  | long double arrow between thick (5mu) spaces |
| `\mathellipsis` |  | the kernel's low ellipsis (\mathinner{\ldotp\ldotp\ldotp}), fontmath.ltx 512 |
| `\bowtie` |  | \triangleright and \triangleleft joined by \joinrel as one relation, fontmath.ltx 366 |
| `\relbar` |  | the single/double arrow shaft as a relation (\mathrel{\smash-} / \mathrel{=}), fontmath.ltx 355-357 |
| `\Relbar` |  | the single/double arrow shaft as a relation (\mathrel{\smash-} / \mathrel{=}), fontmath.ltx 355-357 |
| `\joinrel` |  | \mathrel{\mkern-3mu}: the kern that joins two relations, fontmath.ltx 353 |
| `\surd` |  | the radical sign as an ordinary symbol ({\mathchar"1270}), fontmath.ltx 242 |
| `\Join` |  | amsfonts \rtimes overprinted on \ltimes (msbm "6F, -13.8mu, "6E) as a relation; requires amsfonts/amssymb |
| `\bot` |  | shared symbol glyph with its own atom class (Ord / Bin) |
| `\bigtriangleup` |  | shared symbol glyph with its own atom class (Ord / Bin) |
| `\mathbin` | `{math}` | argument boxed as one atom of the forced class |
| `\mathrel` | `{math}` | argument boxed as one atom of the forced class |
| `\mathord` | `{math}` | argument boxed as one atom of the forced class |
| `\mathop` | `{math}` | argument boxed as one atom of the forced class |
| `\mathopen` | `{math}` | argument boxed as one atom of the forced class |
| `\mathclose` | `{math}` | argument boxed as one atom of the forced class |
| `\mathpunct` | `{math}` | argument boxed as one atom of the forced class |
| `\dag` |  | latex.ltx `{\dagger}`/`{\ddagger}` in math: the cmsy mark as an ordinary atom |
| `\ddag` |  | latex.ltx `{\dagger}`/`{\ddagger}` in math: the cmsy mark as an ordinary atom |
| `\mathbf` | `{text}` | literal text in Times-Bold |
| `\textbf` | `{text}` | literal text in Times-Bold |
| `\mathit` | `{...}` | letters and digits of a plain argument as Unicode mathematical italic, sans-serif or monospace; any other argument stays in the current math face |
| `\mathsf` | `{...}` | letters and digits of a plain argument as Unicode mathematical italic, sans-serif or monospace; any other argument stays in the current math face |
| `\mathtt` | `{...}` | letters and digits of a plain argument as Unicode mathematical italic, sans-serif or monospace; any other argument stays in the current math face |
| `\mathrm` | `{...}` | keeps its argument in the current math face (no distinct face yet) |
| `\mathnormal` | `{...}` | keeps its argument in the current math face (no distinct face yet) |
| `\boldsymbol` | `{...}` | keeps its argument in the current math face (no distinct face yet) |
| `\bm` | `{...}` | keeps its argument in the current math face (no distinct face yet) |
| `\mbox` | `{...}` | keeps its argument in the current math face (no distinct face yet) |
| `\hbox` | `{...}` | keeps its argument in the current math face (no distinct face yet) |
| `\textrm` | `{...}` | keeps its argument in the current math face (no distinct face yet) |
| `\textit` | `{...}` | keeps its argument in the current math face (no distinct face yet) |
| `\textnormal` | `{...}` | keeps its argument in the current math face (no distinct face yet) |
| `\text` | `{text}` | literal text in math |
| `\boxed` | `{...}` | real rule around, over or under the body (underbar works in math like underline) |
| `\overline` | `{...}` | real rule around, over or under the body (underbar works in math like underline) |
| `\underline` | `{...}` | real rule around, over or under the body (underbar works in math like underline) |
| `\underbar` | `{...}` | real rule around, over or under the body (underbar works in math like underline) |
| `\Aboxed` | `{lhs rel rhs}` | mathtools: real \boxed rule around the whole row, keeping the relation as the shared alignment point |
| `\overbrace` | `{body}` | cmex brace pieces with rule fills over or under a display-style body; scripts are limits |
| `\underbrace` | `{body}` | cmex brace pieces with rule fills over or under a display-style body; scripts are limits |
| `\overrightarrow` | `{body}` | amsmath \arrowfill@ as wide as the body, over or under it |
| `\overleftarrow` | `{body}` | amsmath \arrowfill@ as wide as the body, over or under it |
| `\overleftrightarrow` | `{body}` | amsmath \arrowfill@ as wide as the body, over or under it |
| `\underrightarrow` | `{body}` | amsmath \arrowfill@ as wide as the body, over or under it |
| `\underleftarrow` | `{body}` | amsmath \arrowfill@ as wide as the body, over or under it |
| `\underleftrightarrow` | `{body}` | amsmath \arrowfill@ as wide as the body, over or under it |
| `\cancel` | `{body}` | cancel package: diagonal line(s) through the body (forward slash, backward slash, or X) |
| `\bcancel` | `{body}` | cancel package: diagonal line(s) through the body (forward slash, backward slash, or X) |
| `\xcancel` | `{body}` | cancel package: diagonal line(s) through the body (forward slash, backward slash, or X) |
| `\dashrightarrow` |  | amsfonts dashed arrow: two msam \dabar@ pieces and a head in one relation |
| `\dasharrow` |  | amsfonts dashed arrow: two msam \dabar@ pieces and a head in one relation |
| `\dashleftarrow` |  | amsfonts dashed arrow: two msam \dabar@ pieces and a head in one relation |
| `\Bbb` | `{A-Z}` | obsolete amsfonts alias of \mathbb |
| `\bold` | `{text}` | obsolete amsfonts alias of \mathbf |
| `\hat` | `{body}` | base-14 accent glyph centred over the body |
| `\bar` | `{body}` | base-14 accent glyph centred over the body |
| `\vec` | `{body}` | base-14 accent glyph centred over the body |
| `\tilde` | `{body}` | base-14 accent glyph centred over the body |
| `\dot` | `{body}` | base-14 accent glyph centred over the body |
| `\ddot` | `{body}` | base-14 accent glyph centred over the body |
| `\acute` | `{body}` | base-14 accent glyph centred over the body |
| `\grave` | `{body}` | base-14 accent glyph centred over the body |
| `\mathring` | `{body}` | base-14 accent glyph centred over the body |
| `\widehat` | `{body}` | cmex successor-chain accent grown to the body (msbm extra-wide form past 2em with amsfonts) |
| `\widetilde` | `{body}` | cmex successor-chain accent grown to the body (msbm extra-wide form past 2em with amsfonts) |
| `\check` | `{body}` | parsed, but no base-14 glyph exists: diagnosed and typeset without a mark |
| `\breve` | `{body}` | parsed, but no base-14 glyph exists: diagnosed and typeset without a mark |
| `\dddot` | `{body}` | amsmath mathop-limits shape: three/four text dots centred above the body |
| `\ddddot` | `{body}` | amsmath mathop-limits shape: three/four text dots centred above the body |
| `\left` |  | consumes the following delimiter, kept at ordinary size |
| `\right` |  | consumes the following delimiter, kept at ordinary size |
| `\big` |  | consumes the following delimiter, kept at ordinary size |
| `\Big` |  | consumes the following delimiter, kept at ordinary size |
| `\bigg` |  | consumes the following delimiter, kept at ordinary size |
| `\Bigg` |  | consumes the following delimiter, kept at ordinary size |
| `\bigl` |  | consumes the following delimiter, kept at ordinary size |
| `\bigr` |  | consumes the following delimiter, kept at ordinary size |
| `\Bigl` |  | consumes the following delimiter, kept at ordinary size |
| `\Bigr` |  | consumes the following delimiter, kept at ordinary size |
| `\biggl` |  | consumes the following delimiter, kept at ordinary size |
| `\biggr` |  | consumes the following delimiter, kept at ordinary size |
| `\Biggl` |  | consumes the following delimiter, kept at ordinary size |
| `\Biggr` |  | consumes the following delimiter, kept at ordinary size |
| `\bigm` |  | consumes the following delimiter, kept at ordinary size |
| `\Bigm` |  | consumes the following delimiter, kept at ordinary size |
| `\biggm` |  | consumes the following delimiter, kept at ordinary size |
| `\Biggm` |  | consumes the following delimiter, kept at ordinary size |
| `\dots` |  | three periods; amsmath's \dots centres before a binary operator or relation and adds its thin spaces (\mdots@@) |
| `\ldots` |  | three periods; amsmath's \dots centres before a binary operator or relation and adds its thin spaces (\mdots@@) |
| `\dotsc` |  | three periods; amsmath's \dots centres before a binary operator or relation and adds its thin spaces (\mdots@@) |
| `\dotso` |  | three periods; amsmath's \dots centres before a binary operator or relation and adds its thin spaces (\mdots@@) |
| `\cdots` |  | three math-axis dots |
| `\dotsb` |  | three math-axis dots |
| `\dotsm` |  | three math-axis dots |
| `\dotsi` |  | three math-axis dots |
| `\iint` |  | repeated integral glyph |
| `\iiint` |  | repeated integral glyph |
| `\lbrace` |  | brace glyph |
| `\rbrace` |  | brace glyph |
| `\quad` |  | 1em/2em math space |
| `\qquad` |  | 1em/2em math space |
| `\displaystyle` |  | accepted without changing size |
| `\textstyle` |  | accepted without changing size |
| `\scriptstyle` |  | accepted without changing size |
| `\scriptscriptstyle` |  | accepted without changing size |
| `\limits` |  | set a named operator's script placement (`MathAtom::limits`) |
| `\nolimits` |  | set a named operator's script placement (`MathAtom::limits`) |
| `\displaylimits` |  | set a named operator's script placement (`MathAtom::limits`) |
| `\nonumber` |  | accepted without effect |
| `\notag` |  | accepted without effect |
| `\middle` |  | accepted without effect |
| `\tag` | `{label}` | (label) two quads after the display; starred form without parentheses |
| `\qedhere` |  | amsthm end-of-proof box for this display line, set flush right by the render pipeline |
| `\begin` | `{env}` | opens a math grid environment |
| `\backslash` |  | the \setminus glyph as an ordinary symbol (fontmath.ltx \mathord at cmsy "6E) |
| `\vert` |  | single/double bar as an ordinary symbol (fontmath.ltx \mathord) |
| `\Vert` |  | single/double bar as an ordinary symbol (fontmath.ltx \mathord) |
| `\lvert` |  | amsmath opening single/double bar (\mathopen: no glue after it) |
| `\lVert` |  | amsmath opening single/double bar (\mathopen: no glue after it) |
| `\rvert` |  | amsmath closing single/double bar (\mathclose: no glue before it) |
| `\rVert` |  | amsmath closing single/double bar (\mathclose: no glue before it) |
| `\ensuremath` | `{math}` | the argument itself (already in math mode) |
| `\kern` | `<dimen> / <glue>` | TeX kern/glue inside a formula: a fixed horizontal space of the operand's natural points (the expansion pass scans it; stretch and shrink are dropped) |
| `\hskip` | `<dimen> / <glue>` | TeX kern/glue inside a formula: a fixed horizontal space of the operand's natural points (the expansion pass scans it; stretch and shrink are dropped) |
| `\hspace` | `{dimension}` | fixed horizontal space inside a formula (latex.ltx's \hskip); em/ex in the text font, other units in points; starred form identical |
| `\hfil` |  | infinite glue inside a formula: nothing, as a formula box is set at its natural width |
| `\hfill` |  | infinite glue inside a formula: nothing, as a formula box is set at its natural width |
| `\hss` |  | infinite glue inside a formula: nothing, as a formula box is set at its natural width |
| `\hfilneg` |  | infinite glue inside a formula: nothing, as a formula box is set at its natural width |
| `\vspace` | `{dimension}` | consumed with a warning: the vertical adjustment after the line is not inserted |
| `\mkern` | `<mu glue>` | math glue in mu (1/18 of the symbol font's quad); plus/minus stretch is read and dropped |
| `\mskip` | `<mu glue>` | math glue in mu (1/18 of the symbol font's quad); plus/minus stretch is read and dropped |
| `\medspace` |  | amsmath 4mu/5mu glue and their negatives (\tmspace); undefined without amsmath |
| `\thickspace` |  | amsmath 4mu/5mu glue and their negatives (\tmspace); undefined without amsmath |
| `\negmedspace` |  | amsmath 4mu/5mu glue and their negatives (\tmspace); undefined without amsmath |
| `\negthickspace` |  | amsmath 4mu/5mu glue and their negatives (\tmspace); undefined without amsmath |
| `\thinspace` |  | kernel .16667em text-font kern, or 3mu once amsmath rebinds them |
| `\negthinspace` |  | kernel .16667em text-font kern, or 3mu once amsmath rebinds them |
| `\hdots` |  | amsmath alias of \ldots (baseline dots) |
| `\rm` |  | LaTeX 2.09 font switch: the rest of the current group as \mathrm/\mathbf/\mathit/\mathsf/\mathtt/\mathcal/\mathnormal (\DeclareOldFontCommand's math branch) |
| `\bf` |  | LaTeX 2.09 font switch: the rest of the current group as \mathrm/\mathbf/\mathit/\mathsf/\mathtt/\mathcal/\mathnormal (\DeclareOldFontCommand's math branch) |
| `\it` |  | LaTeX 2.09 font switch: the rest of the current group as \mathrm/\mathbf/\mathit/\mathsf/\mathtt/\mathcal/\mathnormal (\DeclareOldFontCommand's math branch) |
| `\sf` |  | LaTeX 2.09 font switch: the rest of the current group as \mathrm/\mathbf/\mathit/\mathsf/\mathtt/\mathcal/\mathnormal (\DeclareOldFontCommand's math branch) |
| `\tt` |  | LaTeX 2.09 font switch: the rest of the current group as \mathrm/\mathbf/\mathit/\mathsf/\mathtt/\mathcal/\mathnormal (\DeclareOldFontCommand's math branch) |
| `\cal` |  | LaTeX 2.09 font switch: the rest of the current group as \mathrm/\mathbf/\mathit/\mathsf/\mathtt/\mathcal/\mathnormal (\DeclareOldFontCommand's math branch) |
| `\mit` |  | LaTeX 2.09 font switch: the rest of the current group as \mathrm/\mathbf/\mathit/\mathsf/\mathtt/\mathcal/\mathnormal (\DeclareOldFontCommand's math branch) |

### Math symbols

`\alpha` α, `\beta` β, `\gamma` γ, `\delta` δ, `\theta` θ, `\lambda` λ, `\mu` μ, `\pi` π, `\sigma` σ, `\phi` φ, `\omega` ω, `\epsilon` ϵ, `\varepsilon` ε, `\zeta` ζ, `\eta` η, `\vartheta` ϑ, `\iota` ι, `\kappa` κ, `\nu` ν, `\xi` ξ, `\varpi` ϖ, `\rho` ρ, `\varsigma` ς, `\tau` τ, `\upsilon` υ, `\varphi` ϕ, `\chi` χ, `\psi` ψ, `\Gamma` Γ, `\Delta` Δ, `\Theta` Θ, `\Lambda` Λ, `\Xi` Ξ, `\Pi` Π, `\Sigma` Σ, `\Upsilon` Υ, `\Phi` Φ, `\Psi` Ψ, `\Omega` Ω, `\le` ≤, `\ge` ≥, `\ne` ≠, `\equiv` ≡, `\sim` ∼, `\cong` ≅, `\propto` ∝, `\perp` ⊥, `\partial` ∂, `\nabla` ∇, `\prod` ∏, `\ast` ∗, `\prime` ′, `\cup` ∪, `\cap` ∩, `\sqcup` ⊔, `\sqcap` ⊓, `\subset` ⊂, `\subseteq` ⊆, `\sqsubseteq` ⊑, `\supset` ⊃, `\supseteq` ⊇, `\sqsupseteq` ⊒, `\notin` ∉, `\ni` ∋, `\emptyset` ∅, `\oplus` ⊕, `\otimes` ⊗, `\ominus` ⊖, `\oslash` ⊘, `\odot` ⊙, `\bigcirc` ◯, `\wedge` ∧, `\land` ∧, `\lor` ∨, `\to` →, `\rightarrow` →, `\leftarrow` ←, `\gets` ←, `\uparrow` ↑, `\downarrow` ↓, `\leftrightarrow` ↔, `\Leftarrow` ⇐, `\Leftrightarrow` ⇔, `\Uparrow` ⇑, `\Downarrow` ⇓, `\aleph` ℵ, `\Re` ℜ, `\Im` ℑ, `\wp` ℘, `\langle` ⟨, `\rangle` ⟩, `\times` ×, `\div` ÷, `\pm` ±, `\leq` ≤, `\geq` ≥, `\neq` ≠, `\not` ̸, `\approx` ≈, `\cdot` ⋅, `\infty` ∞, `\sum` ∑, `\int` ∫, `\in` ∈, `\forall` ∀, `\exists` ∃, `\vee` ∨, `\Rightarrow` ⇒, `\mid` ∣, `\setminus` ∖, `\Longrightarrow` ⟹, `\mp` ∓, `\ll` ≪, `\gg` ≫, `\simeq` ≃, `\vdots` ⋮, `\ddots` ⋱, `\lfloor` ⌊, `\rfloor` ⌋, `\lceil` ⌈, `\rceil` ⌉, `\oint` ∮, `\mapsto` ↦, `\ell` ℓ, `\circ` ∘, `\dagger` †, `\ddagger` ‡, `\mathsection` §, `\S` §, `\mathparagraph` ¶, `\P` ¶, `\parallel` ∥, `\coloneqq` ≔, `\hookrightarrow` ↪, `\models` ⊨, `\vdash` ⊢, `\dashv` ⊣, `\top` ⊤, `\Box` □, `\diamond` ⋄, `\Longleftrightarrow` ⟺, `\longrightarrow` ⟶, `\longleftarrow` ⟵, `\Longleftarrow` ⟸, `\longleftrightarrow` ⟷, `\triangle` △, `\bigtriangledown` ▽, `\varrho` ϱ, `\bullet` ∙, `\prec` ≺, `\succ` ≻, `\preceq` ⪯, `\succeq` ⪰, `\coprod` ∐, `\bigvee` ⋁, `\bigwedge` ⋀, `\biguplus` ⨄, `\bigcap` ⋂, `\bigcup` ⋃, `\bigotimes` ⨂, `\bigoplus` ⨁, `\bigodot` ⨀, `\bigsqcup` ⨆, `\longmapsto` ⟼, `\mathdollar` $, `\leftharpoonup` ↼, `\leftharpoondown` ↽, `\rightharpoonup` ⇀, `\rightharpoondown` ⇁, `\triangleright` ▷, `\triangleleft` ◁, `\ldotp` ., `\star` ⋆, `\flat` ♭, `\natural` ♮, `\sharp` ♯, `\smile` ⌣, `\frown` ⌢, `\imath` 𝚤, `\jmath` 𝚥, `\cdotp` ⋅, `\asymp` ≍, `\nearrow` ↗, `\searrow` ↘, `\nwarrow` ↖, `\swarrow` ↙, `\owns` ∋, `\varbigtriangleup` △, `\varbigtriangledown` ▽, `\lnot` ¬, `\neg` ¬, `\uplus` ⊎, `\arrowvert` ⏐, `\updownarrow` ↕, `\Updownarrow` ⇕, `\wr` ≀, `\amalg` ⨿, `\smallint` ∫, `\clubsuit` ♣, `\diamondsuit` ♢, `\heartsuit` ♡, `\spadesuit` ♠, `\lgroup` ⟮, `\rgroup` ⟯, `\bracevert` ⎪, `\ointop` ∮, `\intop` ∫, `\boxdot` ⊡, `\boxplus` ⊞, `\boxtimes` ⊠, `\square` □, `\blacksquare` ■, `\centerdot` ⬝, `\lozenge` ◊, `\blacklozenge` ⧫, `\circlearrowright` ↻, `\circlearrowleft` ↺, `\leftrightharpoons` ⇋, `\boxminus` ⊟, `\Vdash` ⊩, `\Vvdash` ⊪, `\vDash` ⊨, `\twoheadrightarrow` ↠, `\twoheadleftarrow` ↞, `\leftleftarrows` ⇇, `\rightrightarrows` ⇉, `\upuparrows` ⇈, `\downdownarrows` ⇊, `\upharpoonright` ↾, `\downharpoonright` ⇂, `\upharpoonleft` ↿, `\downharpoonleft` ⇃, `\rightarrowtail` ↣, `\leftarrowtail` ↢, `\leftrightarrows` ⇆, `\rightleftarrows` ⇄, `\Lsh` ↰, `\Rsh` ↱, `\rightsquigarrow` ⇝, `\leftrightsquigarrow` ↭, `\looparrowleft` ↫, `\looparrowright` ↬, `\circeq` ≗, `\succsim` ≿, `\gtrsim` ≳, `\gtrapprox` ⪆, `\multimap` ⊸, `\therefore` ∴, `\because` ∵, `\doteqdot` ≑, `\triangleq` ≜, `\precsim` ≾, `\lesssim` ≲, `\lessapprox` ⪅, `\eqslantless` ⪕, `\eqslantgtr` ⪖, `\curlyeqprec` ⋞, `\curlyeqsucc` ⋟, `\preccurlyeq` ≼, `\leqq` ≦, `\leqslant` ⩽, `\lessgtr` ≶, `\backprime` ‵, `\risingdotseq` ≓, `\fallingdotseq` ≒, `\succcurlyeq` ≽, `\geqq` ≧, `\geqslant` ⩾, `\gtrless` ≷, `\vartriangleright` ⊳, `\vartriangleleft` ⊲, `\trianglerighteq` ⊵, `\trianglelefteq` ⊴, `\bigstar` ★, `\between` ≬, `\blacktriangledown` ▾, `\blacktriangleright` ▶, `\blacktriangleleft` ◀, `\vartriangle` ▵, `\blacktriangle` ▴, `\triangledown` ▿, `\eqcirc` ≖, `\lesseqgtr` ⋚, `\gtreqless` ⋛, `\lesseqqgtr` ⪋, `\gtreqqless` ⪌, `\Rrightarrow` ⇛, `\Lleftarrow` ⇚, `\veebar` ⊻, `\barwedge` ⊼, `\doublebarwedge` ⩞, `\measuredangle` ∡, `\sphericalangle` ∢, `\varpropto` ∝, `\smallsmile` ⌣, `\smallfrown` ⌢, `\Subset` ⋐, `\Supset` ⋑, `\Cup` ⋓, `\Cap` ⋒, `\curlywedge` ⋏, `\curlyvee` ⋎, `\leftthreetimes` ⋋, `\rightthreetimes` ⋌, `\subseteqq` ⫅, `\supseteqq` ⫆, `\bumpeq` ≏, `\Bumpeq` ≎, `\lll` ⋘, `\ggg` ⋙, `\circledS` Ⓢ, `\pitchfork` ⋔, `\dotplus` ∔, `\backsim` ∽, `\backsimeq` ⋍, `\complement` ∁, `\intercal` ⊺, `\circledcirc` ⊚, `\circledast` ⊛, `\circleddash` ⊝, `\lvertneqq` ≨, `\gvertneqq` ≩, `\nleq` ≰, `\ngeq` ≱, `\nless` ≮, `\ngtr` ≯, `\nprec` ⊀, `\nsucc` ⊁, `\lneqq` ≨, `\gneqq` ≩, `\nleqslant` ⩽̸, `\ngeqslant` ⩾̸, `\lneq` ⪇, `\gneq` ⪈, `\npreceq` ⋠, `\nsucceq` ⋡, `\precnsim` ⋨, `\succnsim` ⋩, `\lnsim` ⋦, `\gnsim` ⋧, `\nleqq` ≦̸, `\ngeqq` ≧̸, `\precneqq` ⪵, `\succneqq` ⪶, `\precnapprox` ⪹, `\succnapprox` ⪺, `\lnapprox` ⪉, `\gnapprox` ⪊, `\nsim` ≁, `\ncong` ≇, `\diagup` ⟋, `\diagdown` ⟍, `\varsubsetneq` ⊊, `\varsupsetneq` ⊋, `\nsubseteqq` ⫅̸, `\nsupseteqq` ⫆̸, `\subsetneqq` ⫋, `\supsetneqq` ⫌, `\varsubsetneqq` ⫋, `\varsupsetneqq` ⫌, `\subsetneq` ⊊, `\supsetneq` ⊋, `\nsubseteq` ⊈, `\nsupseteq` ⊉, `\nparallel` ∦, `\nmid` ∤, `\nshortmid` ∤, `\nshortparallel` ∦, `\nvdash` ⊬, `\nVdash` ⊮, `\nvDash` ⊭, `\nVDash` ⊯, `\ntrianglerighteq` ⋭, `\ntrianglelefteq` ⋬, `\ntriangleleft` ⋪, `\ntriangleright` ⋫, `\nleftarrow` ↚, `\nrightarrow` ↛, `\nLeftarrow` ⇍, `\nRightarrow` ⇏, `\nLeftrightarrow` ⇎, `\nleftrightarrow` ↮, `\divideontimes` ⋇, `\nexists` ∄, `\Finv` Ⅎ, `\Game` ⅁, `\eth` ð, `\eqsim` ≂, `\beth` ℶ, `\gimel` ℷ, `\daleth` ℸ, `\lessdot` ⋖, `\gtrdot` ⋗, `\ltimes` ⋉, `\rtimes` ⋊, `\shortmid` ∣, `\shortparallel` ∥, `\smallsetminus` ∖, `\thicksim` ∼, `\thickapprox` ≈, `\approxeq` ≊, `\succapprox` ⪸, `\precapprox` ⪷, `\curvearrowleft` ↶, `\curvearrowright` ↷, `\digamma` ϝ, `\varkappa` ϰ, `\Bbbk` 𝕜, `\hslash` ℏ, `\backepsilon` ϶, `\ulcorner` ⌜, `\urcorner` ⌝, `\llcorner` ⌞, `\lrcorner` ⌟, `\rightleftharpoons` ⇌, `\angle` ∠, `\hbar` ℏ, `\sqsubset` ⊏, `\sqsupset` ⊐, `\mho` ℧, `\yen` ¥, `\checkmark` ✓, `\circledR` ®, `\maltese` ✠, `\restriction` ↾, `\Doteq` ≑, `\doublecup` ⋓, `\doublecap` ⋒, `\llless` ⋘, `\gggtr` ⋙.

### Operator names

Typeset as upright words: `\sin`, `\cos`, `\tan`, `\cot`, `\sec`, `\csc`, `\arcsin`, `\arccos`, `\arctan`, `\sinh`, `\cosh`, `\tanh`, `\coth`, `\log`, `\ln`, `\lg`, `\exp`, `\lim`, `\liminf`, `\limsup`, `\max`, `\min`, `\sup`, `\inf`, `\det`, `\gcd`, `\deg`, `\dim`, `\ker`, `\arg`, `\hom`, `\Pr`, `\sgn`.

### Environments

| Environment | Mode | Behaviour |
| --- | --- | --- |
| `document` | text | the typeset body; preamble content is not typeset |
| `letter` | text | letter.cls: one letter to {recipient\\address}, starting a new page |
| `equation` | text | numbered display |
| `equation*` | text | unnumbered display |
| `displaymath` | text | unnumbered display |
| `gather` | text | centred rows, each numbered |
| `gather*` | text | centred rows |
| `align` | text | rows aligned at &, each numbered |
| `align*` | text | rows aligned at & |
| `alignat` | text | rows aligned at &, each numbered; the column count is consumed |
| `alignat*` | text | rows aligned at &; the column count is consumed |
| `flalign` | text | rows aligned at &, each numbered |
| `flalign*` | text | rows aligned at & |
| `eqnarray` | text | three columns (right, centred, left) with 2\arraycolsep gaps, each row numbered |
| `eqnarray*` | text | three columns (right, centred, left) with 2\arraycolsep gaps |
| `multline` | text | multi-line display; only the last line is numbered |
| `multline*` | text | multi-line display |
| `subequations` | text | amsmath: displays inside number as the parent number plus a, b, ...; a \label right after \begin gets the parent number |
| `figure` | text | numbered captions; no floating (under beamer an in-flow centred box with an unnumbered \small caption) |
| `table` | text | numbered captions; no floating (under beamer an in-flow centred box with an unnumbered \small caption) |
| `block` | text | beamer block: the \large title in the structure colour, the body below it at the enclosing width, \medskip above and \smallskip below; needs \documentclass{beamer} |
| `alertblock` | text | beamer alert block: like block with the title in red; needs \documentclass{beamer} |
| `exampleblock` | text | beamer example block: like block with the title in green; needs \documentclass{beamer} |
| `columns` | text | beamer columns row: \column{width} or column environments set side by side across the paper width ([onlytextwidth]/[totalwidth=] across the text width), [c\|t\|T\|b] alignment; needs \documentclass{beamer} |
| `column` | text | beamer column (environment form of \column): [c\|t\|T\|b]{width}; needs \documentclass{beamer} |
| `frame` | text | rule-bordered box around its body (\fboxsep padding, \fboxrule rule in the current colour); under \documentclass{beamer} a slide: one page per frame (empty frames included), the [t]/[c]/[b] body placement, a {title}{subtitle} head or \frametitle in the body; overlay specs multiply the slides, [fragile] keeps verbatim/lstlisting bodies, [plain] drops the head/foot and [allowframebreaks] splits the body over pages with I/II/... title suffixes |
| `uncoverenv` | text | beamer <overlay> environment: the body keeps its space and is not painted on the slides the spec does not select; needs \documentclass{beamer} |
| `onlyenv` | text | beamer <overlay> environment: the body is typeset only on the slides the spec selects; needs \documentclass{beamer} |
| `visibleenv` | text | beamer <overlay> environment: like uncoverenv; needs \documentclass{beamer} |
| `invisibleenv` | text | beamer <overlay> environment: the body is covered on the slides the spec selects; needs \documentclass{beamer} |
| `alertenv` | text | beamer <overlay> environment: the body in the alert colour on the slides the spec selects; needs \documentclass{beamer} |
| `actionenv` | text | beamer <overlay> environment: with a plain spec, uncoverenv; needs \documentclass{beamer} |
| `tcolorbox` | text | tcolorbox with colback/colframe only, sized to its content like \fcolorbox (0.5mm rule, 1mm padding, black!5!white fill, black!75!white frame); other keys warn and are ignored, corners stay square, no title, one-line bodies only |
| `center` | text | centred paragraphs |
| `flushleft` | text | left-aligned paragraphs |
| `flushright` | text | right-aligned paragraphs |
| `quote` | text | indented paragraphs |
| `quotation` | text | indented paragraphs |
| `sloppypar` | text | a paragraph set with \sloppy |
| `samepage` | text | \samepage for the body |
| `CJK` | text | CJKutf8: \begin{CJK}{UTF8}{family} sets the body's CJK characters as 1 em boxes with the family's subfont metrics, \CJKglue between them and CJK.enc's punctuation no-break rules (needs CJKutf8) |
| `CJK*` | text | CJKutf8: the CJK environment with a source blank after each CJK character ignored (needs CJKutf8) |
| `tiny` | text | the tiny size for the environment body |
| `scriptsize` | text | the scriptsize size for the environment body |
| `footnotesize` | text | the footnotesize size for the environment body |
| `small` | text | the small size for the environment body |
| `normalsize` | text | the body size for the environment body |
| `large` | text | the large size for the environment body |
| `Large` | text | the Large size for the environment body |
| `LARGE` | text | the LARGE size for the environment body |
| `huge` | text | the huge size for the environment body |
| `Huge` | text | the Huge size for the environment body |
| `verse` | text | indented lines; each \\ ends a line |
| `tabbing` | text | tab stops: \= sets a stop, \> jumps right, \\ ends a row, \kill ends a row silently (\<, \+ and \- warn and are ignored) |
| `itemize` | text | bulleted list; article labels per depth, \item[label] |
| `enumerate` | text | numbered list; article labels per depth, enumitem label/label*/shortlabels, start and resume |
| `description` | text | list of bold \item[term] labels |
| `list` | text | kernel list with {default-label}{declarations}; item, item[label], nesting, leftmargin/labelsep/itemsep/topsep |
| `tabular` | text | table with l/c/r/p columns, rules and multicolumn; with array also >{} <{} !{} m b w and \extrarowheight; with siunitx S[options] number and s unit columns, centred rather than decimal-aligned |
| `tabular*` | text | table of a given width |
| `tabularx` | text | table of a given width whose X columns share the leftover width evenly (needs tabularx) |
| `longtable` | text | page-breaking table with repeated heads and feet (\endfirsthead, \endhead, \endfoot, \endlastfoot), \caption, \kill rows and \\* (needs longtable) |
| `verbatim` | text | literal monospaced lines |
| `verbatim*` | text | literal monospaced lines with visible spaces |
| `alltt` | text | monospaced lines with significant spaces and line breaks; commands and groups remain active |
| `lstlisting` | text | literal monospaced lines (basic listings) |
| `comment` | text | body discarded unread, even invalid commands inside (comment package) |
| `proof` | text | amsthm proof with a closing square |
| `thebibliography` | text | References section with numbered \bibitem entries |
| `mcitethebibliography` | text | References section like thebibliography (mciteplus; its sublist grouping is not applied) |
| `multicols` | text | multicol {n}[preface][premulticols]: balanced columns, laid out by the render pipeline |
| `multicols*` | text | multicol {n}[preface][premulticols]: unbalanced columns, laid out by the render pipeline |
| `array` | math | math grid, centred cells |
| `matrix` | math | math grid, centred cells |
| `smallmatrix` | math | math grid, centred cells |
| `pmatrix` | math | math grid, centred cells in ( ) |
| `bmatrix` | math | math grid, centred cells in [ ] |
| `Bmatrix` | math | math grid, centred cells in { } |
| `vmatrix` | math | math grid, centred cells in \| \| |
| `Vmatrix` | math | math grid, centred cells in ‖ ‖ |
| `cases` | math | math grid, left-aligned cells with a left { |
| `dcases` | math | math grid, left-aligned cells with a left { |
| `rcases` | math | math grid, left-aligned cells with a right } |
| `aligned` | math | math grid, centred cells |
| `alignedat` | math | math grid, centred cells |
| `split` | math | math grid, centred cells |
| `gathered` | math | math grid, centred cells |

### Packages loaded without a warning

| Package | Options | Why it is silent |
| --- | --- | --- |
| `alltt` | `` | typewriter lines preserve spaces and line breaks while commands and groups remain active |
| `inputenc` | `utf8` | source text is already decoded as UTF-8 |
| `fontenc` | `T1` | text glyphs are mapped from Unicode |
| `hyperref` | `colorlinks, hidelinks, bookmarks, bookmarksopen, bookmarksnumbered, linktoc, breaklinks, unicode, pageanchor, hyperfootnotes, pdfstartview, pdfpagemode` | loading hyperref moves no glyph (measured against pdflatex, TeX Live 2025: the same document with and without it is 1062 words on 4 pages, 0 moved), and the link-colour, border, outline, viewer and pdf* metadata keys are accepted with it; \url, \href and \nolinkurl are typeset, while the PDF links, bookmarks and link colours still missing are reported once by their own diagnostic; backref and pagebackref add bibliography text and keep warning |
| `cleveref` | `capitalise, noabbrev` | named cross-references with compressed ranges; unknown package options are silently ignored |
| `color` | `dvipsnames, usenames` | color.sty colours with pdfTeX's exact operator values |
| `xcolor` | `natural, rgb, cmy, cmyk, gray, dvipsnames, svgnames, x11names, table` | xcolor 3.02 definitions, expressions and target models with pdfTeX's exact operator values; hsb models, colour series and table colours are diagnosed |
| `amsmath` | `centertags, sumlimits, nointlimits, namelimits, reqno` | the align, gather, multline, split, aligned, gathered, cases and matrix families; \dfrac, \tfrac, \binom, \genfrac, \cfrac, \substack, \operatorname, \DeclareMathOperator, \boxed, \phantom, \overset/\underset, the extensible arrows, \text in math, \tag/\notag and \eqref, \sideset, with \lim-family, \sum and \prod display limits and amsmath's wider \colon. Its defaults are the accepted options; leqno, fleqn, tbtags, nosumlimits, intlimits and nonamelimits move real output and keep warning. \shoveleft, \smash, \mspace, \hdotsfor and \varinjlim are each diagnosed where they are used |
| `amssymb` | `` | the full AMSa/AMSb (msam/msbm) inventory of amssymb.sty -- 203 names base LaTeX2e leaves undefined (\square, \nleq, ...) -- plus everything amsfonts declares; loading the package is what makes the names exist, and a name whose file was not loaded is diagnosed |
| `amsfonts` | `` | amsfonts.sty's 22-name symbol subset (\ulcorner, \square, \yen, the dashed arrows) and the \mathbb and \mathfrak alphabets; the rest of amssymb stays undefined without \usepackage{amssymb} |
| `amsthm` | `` | \newtheorem, \theoremstyle and the proof environment |
| `array` | `` | tabular >{} <{} !{} m b w columns, \newcolumntype and \extrarowheight |
| `tabularx` | `` | the tabularx environment and its X column, splitting the table's leftover width evenly |
| `booktabs` | `` | \toprule, \midrule, \bottomrule, \cmidrule(trim), \addlinespace, \specialrule, \morecmidrules |
| `cancel` | `` | \cancel (forward diagonal), \bcancel (backward diagonal) and \xcancel (X) through a math expression; \cancelto is diagnosed |
| `longtable` | `` | the page-breaking longtable environment: \endfirsthead, \endhead, \endfoot, \endlastfoot, \caption, \kill, \\* |
| `multirow` | `` | \multirow[vpos]{rows}[bigstruts]{width}[vmove]{text} in table entries |
| `colortbl` | `` | \rowcolor, \cellcolor, >{\columncolor}, \arrayrulecolor, \doublerulesepcolor |
| `enumitem` | `shortlabels` | list keys (label, start, resume, seps, margins) parsed as options; \setlist |
| `geometry` | `letterpaper, margin=1in` | matches the fixed US Letter page with 1in margins |
| `siunitx` | `any \sisetup keys` | v3 \num, \unit, \qty, lists, ranges, \ang, \sisetup and \DeclareSIUnit; unmodelled keys are diagnosed |
| `multicol` | `` | multicols and multicols* with preface, \columnbreak, \raggedcolumns (columns set by the render pipeline) |
| `natbib` | `numbers, authoryear, round, square, angle, curly, comma, semicolon, colon, nobibstyle, bibstyle, sectionbib, longnamesfirst, nonamebreak` | \citet/\citep/\citealt/\citealp/\citeauthor/\citeyear/\citeyearpar/\citenum/\citetext and the \cite it redefines, with [Author(Year)] \bibitem labels; sort, compress, super and openbib are diagnosed |
| `cite` | `space, nospace, nosort, nocompress, sort, compress, adjust, move, verbose` | \cite sorts numeric keys, compresses three or more consecutive numbers into a range and separates entries with cite.sty's \citepunct glue; superscript, noadjust, nomove, nobreak, ref and biblabel are diagnosed |
| `biblatex` | `style=numeric, sorting=none, backend=biber` | basic project-relative .bib resources with numeric citations, textcite/parencite/autocite, citeauthor/citeyear, nocite and printbibliography; authoryear labels are minimal, alphabetic warns |
| `ulem` | `normalem` | \uline: 0.4pt rule under the argument (single-line); \sout: 0.4pt strike at 0.55ex; \emph is not redefined |
| `soul` | `` | \so: letterspaced argument (0.25em between letters, 0.65em word spaces, 0.55em at the edges, single-line); \hl: yellow behind-text rule at natural width, 1.75ex above and 0.75ex below the baseline (single-line; interword gaps between fragments are not painted, see GH-828); \st stays unsupported |
| `relsize` | `` | \larger/\smaller step the size in effect by an optional [n] (default 1), relative to the closest defined size |
| `fancyhdr` | `` | \pagestyle{fancy} ships the \fancyhead/\fancyfoot fields ([LE,RO]-style positions; a group with E but not O never ships one-sided) with the 0.4pt head rule; \fancyhf clears all six fields; \lhead/\chead/\rhead and \lfoot/\cfoot/\rfoot set one field each (an optional even-page group is ignored one-sided); \fancypagestyle is diagnosed where it is used |
| `titlesec` | `` | \titleformat{\section} headings take the format's face and size (unnumbered with an empty label) with the \titlerule after-code rule; other levels, printed labels, before-code and shapes beyond the implemented subset are diagnosed where they are used |
| `tcolorbox` | `` | the tcolorbox environment with colback/colframe only (see the tcolorbox environment); every other key and every library option is diagnosed |
| `xspace` | `` | \xspace inserts a word space unless the next token is }, , . ' / ? ; : ! ~ - ), or a short suppressing-command list (\footnote, \footnotemark, \bgroup, \egroup, control space) |
| `ifthen` | `` | \ifthenelse with \equal, \NOT, \AND, \OR, \isodd, \isundefined, \lengthtest and \boolean tests, and \newif conditionals with \newboolean/\setboolean; \whiledo loops are diagnosed where they are used |
| `csquotes` | `` | \enquote: typographic quotation marks, alternating double/single on nesting |
| `CJKutf8` | `` | the CJK and CJK* environments with the UTF8 encoding and the min, goth, maru, gbsn, gkai, bsmi, bkai and mj families: each CJK character is a 1 em box with the family's subfont height and depth, \CJKglue (0pt plus 0.08\baselineskip) between characters and CJK.enc's no-break rules around punctuation; painted from an installed CJK font (Hiragino, Songti, ...) named in one diagnostic; \CJKfamily, \CJKspace, \CJKnospace and \CJKtilde; other encodings and families, vertical text and CJKpunct are diagnosed |
| `CJK` | `encapsulated` | the package CJKutf8 loads; accepted with the same environment and commands (the body is read as UTF-8 either way) |
| `calc` | `` | \setlength/\addtolength accept +/- chains of dimensions (1pt + 2\baselineskip); *, /, parentheses and \widthof/\heightof/\depthof/\totalheightof are not parsed |
| `etoolbox` | `` | toggle booleans: \newtoggle/\providetoggle declare a false toggle, \toggletrue/\togglefalse set it, \iftoggle{name}{true}{false} selects a branch at expansion time; a duplicate \newtoggle and any use of an undefined toggle are diagnosed where they are used and leave existing state alone. The rest of etoolbox (patching, hooks, list processing) is diagnosed where it is used |
| `iftex` | `` | \ifxetex and \ifluatex (with the \ifXeTeX/\ifLuaTeX aliases) are false, as iftex.sty sets them under pdflatex, so engine-guarded blocks skip |
| `ifxetex` | `` | legacy shim for iftex's \ifxetex switch, false here as under pdflatex |
| `ifluatex` | `` | legacy shim for iftex's \ifluatex switch, false here as under pdflatex |
| `parskip` | `` | \parindent 0pt and \parskip of half the class \baselineskip (6.0pt at 10pt, 6.8pt at 11pt, 7.25pt at 12pt; the plus 2pt stretch is not modelled); package options are diagnosed |

Any other package, or these packages with other options, is recorded and reported as recognised but not implemented.

### Project `.sty` and `.cls` files

`\usepackage{name}`, `\RequirePackage`, `\documentclass` and `\LoadClass` read `name.sty`/`name.cls` from the project (next to the entry document, then at the project root) and run it through the expansion engine: `\ProvidesPackage`, `\DeclareOption`, `\ProcessOptions`, `\PassOptionsToPackage`, `\RequirePackage`, `\LoadClass`, `\AtEndOfPackage`, the `\@if...` queries and the `\Package...`/`\Class...` messages behave as in latex.ltx, and `@` is a letter while the file is read. A class file's `\LoadClass{article|report|book|letter|beamer}` gives the document that class's page model; a class with no `\LoadClass` gets `article`'s with a warning. The packages and classes below are modelled by the typesetter and are never read from a project file, even when one of that name exists:

| Built-in package | Why its file is not executed |
| --- | --- |
| `amsmath` | displays, \dfrac/\binom/\operatorname and the align family are parsed and laid out by crate::math; amsmath.sty needs \halign, \setbox and \mathchoice |
| `amssymb` | the msam/msbm symbol inventory is a table (crate::amssymb); amssymb.sty needs \DeclareMathSymbol on real font encodings |
| `amsfonts` | the amsfonts subset and \mathbb/\mathfrak are tables; the file needs \DeclareFontFamily |
| `amsthm` | \newtheorem, \theoremstyle and proof are crate::theorems; amsthm.sty needs \hbox and \vskip |
| `mathtools` | amsmath extensions parsed by crate::math; the file needs \setbox and \mathchoice |
| `bm` | \bm is a bold math switch in crate::math; bm.sty needs \mathchardef tables and \font |
| `physics` | \dv, \pdv, \abs & co. are parsed by crate::math; the file needs \mathchoice |
| `cancel` | \cancel/\bcancel/\xcancel are crate::math frames; cancel.sty needs \hbox and \vrule |
| `siunitx` | numbers, units and quantities are crate::siunitx; siunitx.sty is expl3 code |
| `mhchem` | parsed by crate::math; the file is expl3 code |
| `geometry` | the page frame is crates/class-geometry; geometry.sty needs \pdfpagewidth and \hsize |
| `fancyhdr` | \pagestyle{fancy} fields are parser state; fancyhdr.sty needs \vbox and \hrule |
| `titlesec` | sectioning shapes are the render pipeline's; titlesec.sty needs \vbox and \hangindent |
| `titling` | title-block hooks are parser state |
| `setspace` | \onehalfspacing/\doublespacing are parser leading state; setspace.sty needs \baselineskip arithmetic |
| `parskip` | \parskip/\parindent are read by the render pipeline from the package name |
| `multicol` | multicols is laid out by the render pipeline; multicol.sty needs \output and \vsplit |
| `enumitem` | list keys are parser state; enumitem.sty needs \hbox and list primitives |
| `microtype` | protrusion and expansion are crates/microtype; microtype.sty needs pdfTeX's \pdfprotrudechars |
| `csquotes` | \enquote is a parser command; csquotes.sty needs \lccode tables and expl3 |
| `xspace` | \xspace is a parser command; xspace.sty needs \futurelet on a space-factor table |
| `relsize` | \larger/\smaller are parser font state; relsize.sty needs \fontdimen |
| `ulem` | \uline/\sout are parser decorations; ulem.sty needs \hbox and \vrule |
| `soul` | \so/\hl are parser decorations; soul.sty needs \hbox and \discretionary |
| `textcomp` | text symbols are the Unicode text tables; the file needs \DeclareTextSymbol |
| `lipsum` | \lipsum text is a parser table |
| `verbatim` | verbatim and comment environments are read by the lexer; verbatim.sty needs \catcode tricks on \obeylines output |
| `comment` | the comment environment is read by the lexer |
| `alltt` | the alltt environment is read by the parser; alltt.sty needs \catcode tricks on \obeylines output |
| `url` | \url is parsed raw by the lexer; url.sty needs \discretionary and \catcode tricks |
| `nameref` | \nameref is crate::xref |
| `inputenc` | source text is decoded as UTF-8; inputenc.sty needs \DeclareInputText on active characters |
| `fontenc` | T1/OT1 are the text encoding tables; fontenc.sty needs \DeclareFontEncoding |
| `lmodern` | Latin Modern is the render pipeline's font set; lmodern.sty needs \DeclareFontFamily |
| `fontspec` | \setmainfont & co. are font settings (proposal S4); fontspec.sty is expl3 code |
| `unicode-math` | `\setmathfont{…}` selects the OpenType math font (and the package alone selects Latin Modern Math); the file is expl3 code |
| `babel` | language selection is not modelled; babel.sty needs \language and \lccode tables |
| `CJKutf8` | the CJK environment, \CJKfamily and the space switches are parser state and the render pipeline sets the characters from the C70 subfont metrics; CJKutf8.sty needs active characters and \lastkern |
| `CJK` | loaded by CJKutf8; CJK.sty needs active characters, \lastkern and \pdffontattr |
| `iftex` | \ifpdftex & co. would misreport the engine; the file tests primitives |
| `ifxetex` | \ifxetex is the parser's; the file tests primitives |
| `ifluatex` | \ifluatex is the parser's; the file tests primitives |
| `calc` | \setlength arithmetic is the engine's \dimexpr; calc.sty needs \dimen registers with \advance semantics |
| `etoolbox` | toggles are the engine's HOST_PRELUDE; etoolbox.sty needs \numexpr on \catcode tables and \afterassignment tricks |
| `ifthen` | \ifthenelse is an engine primitive |
| `array` | column types and the row strut are crate::tabular; array.sty needs \halign |
| `tabularx` | X columns are crate::tabular; tabularx.sty needs \setbox and \halign |
| `booktabs` | rules are crate::tabular; booktabs.sty needs \hrule and \noalign |
| `longtable` | page-breaking tables are crate::tabular; longtable.sty needs \output |
| `multirow` | multirow entries are crate::tabular; multirow.sty needs \vbox |
| `colortbl` | cell colours are crate::tabular; colortbl.sty needs \noalign and \leaders |
| `caption` | caption shapes are the render pipeline's; caption.sty needs \hbox and \vbox |
| `subcaption` | subfigures are the render pipeline's; the file needs caption's machinery |
| `float` | [H] placement is the render pipeline's; float.sty needs \output |
| `wrapfig` | wrapped figures are the render pipeline's; wrapfig.sty needs \parshape and \output |
| `tcolorbox` | the tcolorbox environment (colback/colframe) is the parser's; tcolorbox.sty needs pgf and \setbox |
| `graphicx` | \includegraphics is the render pipeline's; graphicx.sty needs \pdfximage and \setbox |
| `graphics` | \includegraphics is the render pipeline's; graphics.sty needs \pdfximage and \setbox |
| `xcolor` | colour models are crate::color; xcolor.sty needs \pdfcolorstack and \special |
| `color` | colour models are crate::color; color.sty needs \special |
| `tikz` | pictures are crates/vector-graphics; tikz.sty and pgf need \pdfliteral and \setbox |
| `pgf` | pgf needs \pdfliteral and \setbox |
| `pgfplots` | pgfplots needs pgf |
| `listings` | listings are the render pipeline's; listings.sty needs \catcode tricks on \obeylines output |
| `hyperref` | links are the PDF writer's; hyperref.sty needs \pdfstartlink and \special |
| `cleveref` | \cref is crate::xref; cleveref.sty patches \refstepcounter with \protected@write |
| `natbib` | citations are crate::natbib; natbib.sty needs \bibitem output |
| `cite` | sorted, range-compressed citations with cite.sty's own separator glue are crate::bib; cite.sty needs \futurelet on the token after \cite and \lastskip/\lastpenalty |
| `biblatex` | citations are crate::biblatex; biblatex.sty is expl3 code |
| `beamerthemedefault` | beamer themes are crates/class-geometry |

| Built-in class | Page model |
| --- | --- |
| `article` | size1x.clo page model in crates/class-geometry |
| `report` | size1x.clo page model in crates/class-geometry |
| `book` | size1x.clo page model in crates/class-geometry |
| `letter` | letter.cls page model in crates/class-geometry |
| `beamer` | beamer's frame model in crates/class-geometry |
| `scrartcl` | KOMA typearea in crates/class-geometry |
| `scrarticle` | KOMA typearea in crates/class-geometry |
| `scrreprt` | KOMA typearea in crates/class-geometry |
| `scrbook` | KOMA typearea in crates/class-geometry |
| `amsart` | AMS size tables in the parser |
| `amsbook` | AMS size tables in the parser |
| `amsproc` | AMS size tables in the parser |
<!-- END GENERATED supported-latex -->

What each project package or class file defined -- its `\newcommand`s,
`\def`s, `\let`s, environments, `\newif` switches, counters, lengths,
theorems, `\DeclareMathOperator`s and `\NewDocumentCommand`s, each with its
parameter shape and the byte span of the defining statement, plus the file's
`\ProvidesPackage` and `\DeclareOption`s and the command that loaded it -- is
reported to the editor in the `metadata.packages` section of the worker's
`compile_result` (schema in
[runtime-v1 › Optional `metadata` object](../contracts/runtime-v1.md#optional-metadata-object)).
Only definitions made at the file's outermost level are listed; a definition
made by a macro the file calls, by an option's code, or by an
`\AtEndOfPackage` hook is not. A diagnostic raised inside a package names the
whole chain of files that loaded it, back to the document.

Through `flashtex-render` (what the `flashtex` CLI and the app run), packages the
render pipeline sets on the compiler's behalf are silent too, with any options:
`amsmath`, `amssymb`, `amsfonts`, `lmodern`, `microtype`, `geometry`, `graphicx`
and `tikz`. The same goes for `\pagestyle` in the preamble of a document with a
`\documentclass`, which the pipeline's page-frame reader takes. Running
`flashtex-compiler` on its own still reports them.

## Troubleshooting

Diagnostics appear on `flashtex`'s stderr, in the app's Problems panel and
in `diagnostics[]`. The common codes and what to do about them:

| Code | Severity | Meaning | What you can do |
|---|---|---|---|
| `missing_file`, `include_cycle`, `path_escapes_root`, `invalid_path`, `not_utf8`, `read_error`, `include_depth`, `unresolved_reference` | error (warning for `unresolved_reference`) | From `flashtex`'s project discovery: an `\input`/`\include`/`\includegraphics` target that does not exist, includes itself, points outside the project root (`..` or a symlink), is not a valid project path, is not UTF-8, cannot be read, nests deeper than the limit, or has an argument needing macro expansion (`\input{\jobname}`) | Fix the path; the build continues without that file (`--strict` turns the errors into exit 1) |
| `compiler` | warning or error | A message from the parser, re-wrapped by `flashtex-render`: `\foo is not supported by this compiler version`, `packages X are recognised but not implemented`, `\setlist keys leftmargin … are recognised but not implemented`, `environment 'X' is not implemented; its body is typeset as plain text`, `undefined reference`, unmatched braces, an `\input` file not found. (`flashtex-compiler` itself emits these without a `code` field.) | Remove or replace the construct; the `recovery` text says what was rendered instead |
| `overfull_hbox` | warning | `overfull line: N pt too wide (no hyphenation available)` — a line could not be broken within the text width, so it sticks into the margin like TeX's *Overfull \hbox* | Rephrase or add a break point; hyphenation is not implemented |
| `overfull_vbox` | warning | A line extends past the page's text area | Shorten the page or force a break with `\newpage` |
| `overfull_display` | warning | Display math is wider than the text width by N pt | Split the formula |
| `paragraph_final_linebreak` | warning | A trailing `\\` at the end of a paragraph was ignored (LaTeX would set an empty last line; this paragraph is one line pitch shorter) | Remove the trailing `\\` |
| `math_resource_profile` | warning (no source) | Glyphs of a TeX math family (`lmmi10`, `lmsy10`, `lmex10`) are drawn from the single-design Latin Modern Math font because no optical-size outline resource exists; outlines differ slightly from the pdfLaTeX design | Informational — positions are still from TeX metrics |
| `math_limitation` | warning | A math construct is laid out with a documented simplification: `align`/`gather` rows set as separate centred displays with `&` ignored, a nested `array`/`cases`/`matrix` inside a sub-formula set as one row, `\quad` glue inside `\left…\right` dropped, a delimiter or radical variant that is too small, a missing math glyph or accent | Informational; restructure if the simplification is visible |
| `math_glyph_unmapped` | warning | TeX metrics placed a glyph (an extensible assembly) that has no Latin Modern Math mapping; nothing was drawn | Use a smaller delimiter |
| `math_text_overflow` | error | Too many `\text{}` arguments in one formula; the extra ones were not typeset | Split the formula |
| `unsupported_block` | error | Historical: a block-level construct without a layout. Current `flashtex-render` builds do not emit it (unknown environments come through as `compiler` warnings instead) | — |
| `unsupported_script` | error | Text in a script the shaper does not handle (non-Latin); the segment was not drawn | Not supported yet |
| `missing_glyph` | warning | A character has no glyph in the selected Latin Modern face; nothing was drawn for it | Use a supported character or `\text{}` with Latin text |
| `font_unavailable` / `math_font_unavailable` | error | The Latin Modern text face (Times metrics substituted) or Latin Modern Math (no math typeset) could not be found | Check the font search path above; with the bundled app this indicates a broken install |
| `tfm_missing` / `math_metrics_opentype` / `tfm_run_error` | warning | A `.tfm` metric file was not found (or its lig/kern program failed on a word), so OpenType advances were used for that face or word | Point `FLASHTEX_TFM_DIRS` at a Latin Modern `texmf/fonts/tfm/public/lm` directory, or use the bundled app, which ships the metrics for 5–17 pt roman, 5–12 pt bold and 7–12 pt italic |
| `required_metrics_unavailable` | error | The pinned required metric set (`ec-lmr12`, `rm-lmr12/8/6`, licence) is missing or altered; layout is not reference geometry | Reinstall; the bundle is verified at packaging time |
| `labels_unstable` | warning (no source) | `\ref`/`\pageref` values did not converge within the pass limit; the last pass is shown | Check for labels whose value depends on their own page position |
| `display_list_declined` | warning (no source) | The `display-list-v2` sibling would exceed the 16 MiB reply limit, so the app's v2 preview receives no frame for this revision (the v1 pages still render) | Split the project into `\input` files |
| `payload_too_large`, `malformed_json`, `invalid_utf8`, `unsupported_protocol_version`, `unsupported_type` | `error` envelope | The request line itself was rejected (over 8 MiB, not JSON, …); no `compile_result` is produced and the worker keeps running | Fix the request; split very large documents |

When a `compile_result` is `recovered`, the pages are real output rendered
around the problem; when it is `failed` there are no pages and the app keeps
the last good preview on screen, with the old underlines flagged as kept from
the previous revision.
