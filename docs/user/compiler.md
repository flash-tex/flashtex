# The `flashtex` command line, and building from source

`flashtex` builds, checks and watches a LaTeX project on FlashTeX's engine,
the same engine the Mac app's preview runs. It is pdfLaTeX-compatible by
default (the PDF it writes is the one pdflatex writes) and
XeLaTeX-compatible in Unicode mode.

```sh
flashtex build main.tex                 # main.pdf next to it: the PDF pdflatex writes
flashtex build paper -o out.pdf         # a directory: its one file with \documentclass, or main.tex
flashtex build main.tex --mode unicode  # Unicode mode (XeTeX): fontspec, unicode-math, system fonts
flashtex build main.tex --no-tools      # without bibtex, biber and makeindex
flashtex check main.tex [--json]        # TeX's diagnostics: file:line:col: severity: message; exit 1 on an error
flashtex watch main.tex [--interval MS] # rebuild on every change, one warm engine; Ctrl-C stops
flashtex build main.tex --no-progress   # no progress line (also: --progress, -q/--quiet)
flashtex licenses                       # the licences and the engine's GPL source offer
flashtex --version
```

`flashtex` (crates/flashtex-build) is MIT and holds no engine code. It
starts an **engine host** as a separate program and talks to it over the
display-list-v3 socket, as the Mac app does (DESIGN.md §3):
`flashtex-host` (crates/flashtex-engine, a translation of pdfTeX) for
Classic mode and `flashtex-host-unicode` (crates/flashtex-xetex, a
translation of XeTeX) for Unicode mode. The hosts are GPL-2.0-or-later; see
[Licences and source](#licences-and-source).

Until retirement stage S7 this program was called `flashtex-v3`, and
`flashtex` was the old engine's command line (crates/flashtex-cli), which
was deleted with that engine (DESIGN.md §10, row D5; RQ6). The old CLI's
`supported`, `worker`, `fonts`, `manifest`, `packages` and `install-cli`
subcommands, its `--font-dir`, `--v2`, `--timing` and `--strict` flags, its
`flashtex-check/1` report and its helper binaries (`flashtex-render`,
`flashtex-compiler`, `flashtex-pdf`, `flashtex-pdf-exact`) went with it.

## What each command does

- `build` compiles as latexmk would. It runs bibtex, biber and makeindex
  when the document needs them, and as many passes as the `.aux` asks for.
  It then writes the PDF of an `export` run, which is byte-identical to
  pdflatex's apart from the dates and the document ID. Only the PDF goes
  into the project: the `.aux`, `.log` and the rest go to a fresh private
  directory under the system's temporary directory (mode 0700 on Unix),
  removed at the end, so concurrent runs never share one. The PDF goes to
  `-o`, else next to the main file.
- `check` compiles and prints TeX's diagnostics (`diag-v1`) as
  `file:line:col: severity: message`, or one JSON object a line with
  `--json`. It also reports the external tools: a failed bibtex, biber or
  makeindex is an error, a tool the document needs but that could not run
  (its database is missing) a warning.
- `watch` keeps one warm host for the session and builds again when a
  source changes (checked every `--interval` ms, default 500, settled for
  200 ms); the PDF it writes is not a source. A save made while a build runs
  is built next, and a host that crashes is started again at the next build.
- `licenses` prints this program's licence, the hosts' licences and where
  their source is.
- `--version` prints `flashtex <version> (<revision>)`.

The target is a `.tex` file, a directory (its one file with
`\documentclass`, or `main.tex`), or nothing (the current directory).

Exit status: 0 on success, 1 on a TeX error or when no PDF was written, 2
for a usage error.

### The mode

The mode picks the host, highest first:

1. `--mode classic|unicode`;
2. `$FLASHTEX_MODE`;
3. the manifest's `[project] mode` ([`flashtex.toml`](project-manifest.md));
4. a `% !TEX program = xelatex` (or `TS-program`) line among the main file's
   leading comments: `xelatex` is Unicode, `xetex` Unicode with plain
   XeTeX's format, `pdflatex` Classic;
5. Classic.

Classic runs `flashtex-host` with the `pdflatex` format; Unicode runs
`flashtex-host-unicode` with `xelatex`. A program FlashTeX does not have
(LuaLaTeX) or a manifest value it does not know is a warning that says what
runs instead. When the mode does not come from the default, `build` says
which mode it chose and why on stderr.

### Finding the host and the TeX files

- The host is `--host PATH`, else `$FLASHTEX_HOST` (Unicode mode:
  `$FLASHTEX_HOST_UNICODE`), else the mode's host program beside
  `flashtex`, else on `PATH`. The engine's string pool is `$FLASHTEX_POOL`,
  else `pdftex.pool` beside the host.
- The host reads the TeX files (formats, classes, packages, fonts) from a
  **TeX Live** installation, through `kpsewhich` (D12). Without one, it can
  use FlashTeX's **TeX bundle**, the pinned TeX Live 2026 tree the Mac app
  downloads: the lock `flashtex-bundle.lock` beside the host (the CLI
  tarball ships it), `$FLASHTEX_BUNDLE_LOCK`, or one in the per-user
  configuration directory (`~/Library/Application Support/FlashTeX/` on
  macOS, `~/.config/flashtex/` elsewhere) names it. Nothing is downloaded
  unless you allow it: `FLASHTEX_BUNDLE_ALLOW_FETCH=1`. After the first
  download it is cached, and `FLASHTEX_BUNDLE_OFFLINE=1` keeps the host from
  fetching anything more.
- When neither is there, the host says so, with what to set.

### Stopping

Ctrl-C or SIGTERM (Windows: Ctrl-C, Ctrl-Break, closing the console) stops
the host and removes the work directory. If `flashtex` is killed outright,
the host it started (`--once`) notices its parent is gone, removes its
socket and exits, and the next run (Unix) removes the work directory that
was left behind (also one a `flashtex-v3` of an older release left).

### Progress

When stderr is a terminal, `build`, `check` and `watch` show one status line
on stderr, redrawn in place about ten times a second, and a summary at the end:

```text
    Starting the engine host  0.7s
   Compiling thesis.tex  pass 2  [===========>                  ] 210/~541 pages  chapters/ch6.tex  8.8s  ETA 5.1s
     Running bibtex thesis.aux
     Writing thesis.pdf  [=====================>        ] 386/541 pages  19.6s  ETA 2.9s
    Finished thesis.pdf — 541 pages, 2 passes, 21.7s
```

- **pass**: the engine's passes over the document, counting a further pass
  for the `.aux` and the passes after bibtex, biber or makeindex (each tool
  has a `Running` line of its own). **Writing** is the export run that
  writes the PDF pdflatex writes.
- **pages**: the pages the pass has shipped out, out of the last build's
  count (`~541`, kept in `flashtex/progress/` in the user's cache
  directory) or, for the export, the exact count. Without a last build, a
  spinner and a count. The ETA is shown when the total is known.
- **the file**: the innermost `\input`, `\include` or package TeX is reading,
  relative to the project.
- **Finished**: the pages, the passes and the time, and the number of
  errors (printed above it) and warnings (`flashtex check` lists them).
- The line fits the terminal's width (it drops the ETA, then the bar, then
  shortens the file name). `NO_COLOR` turns the colour off; `TERM=dumb`
  turns the line off.
- **Not a terminal** (a pipe, CI, a script): no progress at all, so the
  output is byte for byte what it was without it (one `flashtex: wrote …`
  line). `--progress` asks for plain lines there instead (`flashtex:
  pass 2`, at most one more every 5 s, and the tools and the export).
  `--no-progress` turns it off on a terminal; `-q`/`--quiet` turns it off and
  drops the line for a successful build (errors and warnings still print).
- Progress never goes to stdout (where `check` prints its diagnostics) and
  costs the build nothing measurable: it comes from the host's `progress-v1`
  heartbeat (display-list-v3 §6.8), sent at page checkpoints at most every
  250 ms, which `flashtex` asks for only when it shows progress.

## Install

The command line is independent of the Mac app; it runs on macOS (Apple
Silicon) and Linux (x86_64):

```sh
curl -fsSL https://flash-tex.github.io/flashtex/install-cli.sh | sh
```

It detects your OS/CPU, downloads the matching
`flashtex-cli-<version>-<platform>.tar.gz` from
[Releases](https://github.com/flash-tex/flashtex/releases), verifies it
against that release's `SHA256SUMS` (refusing on a mismatch), and installs
`flashtex`, its engine hosts and what they read beside them into
`~/.local/bin`, and the licences, the source offer and the README into
`~/.local/share/flashtex`. Flags: `--prefix DIR` (e.g. `/usr/local`, may
need sudo), `--version vX.Y.Z` (default: the latest release),
`--uninstall`. It also removes an older release's helpers
(`flashtex-render` and the rest) and fonts. For the Mac app instead (a
SwiftUI IDE on the same engine; it does not bundle this CLI), see
[Getting started](README.md#install).

The tarball, extracted by hand, works where it is; keep `bin/` together:

| Path | What |
|---|---|
| `bin/flashtex` | The CLI (MIT) |
| `bin/flashtex-host` | Classic mode's engine host (GPL-2.0-or-later) |
| `bin/flashtex-host-unicode` | Unicode mode's engine host (GPL-2.0-or-later), on macOS |
| `bin/pdftex.pool` | `flashtex-host`'s string pool |
| `bin/flashtex-bundle.lock` | The pinned TeX bundle, for a machine without TeX Live |
| `share/flashtex/LICENSE` | `flashtex`'s licence (MIT) |
| `share/flashtex/engine/` | The hosts' licences and the notices of what they link |
| `SOURCE.md` | Where the source of all of it is |

## Licences and source

`flashtex` is MIT. The engine hosts are free software under the GNU General
Public License, version 2 or later; they link TeX Live 2026's kpathsea,
zlib, libpng and xpdf (xpdf is GPL version 2 or 3 only, so the hosts are
distributed under GPL version 2 or 3), and Unicode mode's host also
HarfBuzz, FreeType, TECkit and ICU's data, vendored under `third_party/`.
Their complete corresponding source is this repository: every release
links the tree at its tag from its notes, attaches it as
`flashtex-<version>-source.tar.gz`, and `flashtex licenses` and the
tarball's `SOURCE.md` name it, as the Mac app's About panel does (RQ19).
`flashtex` never links a host, and no MIT crate may depend on one
(`scripts/check-license-boundary.sh`).

## Building from source

Requirements: a stable Rust toolchain from rustup (the CLI builds on macOS
and Linux), and TeX Live to typeset (or the bundle, above); for the Mac app,
macOS 14+ on Apple Silicon and Xcode Command Line Tools (Swift 6). Every
crate under `crates/` except `crates/flashtex-xetex` is a member of the root
Cargo workspace, so binaries land in the repository's `target/release/`
(resolve it with `scripts/crate-target-dir.sh <crate-dir>` rather than
hard-coding it):

```sh
git clone https://github.com/flash-tex/flashtex.git && cd flashtex
cargo build --release -p flashtex-build --bin flashtex          # the CLI
cargo build --release -p flashtex-engine --bin flashtex-host    # Classic mode's host, beside it
target/release/flashtex build main.tex -o /tmp/main.pdf        # needs TeX Live, or the bundle
(cd crates/flashtex-xetex && cargo build --release --bin flashtex-host-unicode)   # Unicode mode's host (its own workspace)
FLASHTEX_HOST_UNICODE=crates/flashtex-xetex/target/release/flashtex-host-unicode \
  target/release/flashtex build main.tex --mode unicode
scripts/ci/package-cli.sh 0.0.0-local macos-arm64 dist   # the CLI tarball, from the built binaries
scripts/ci/build-helpers.sh                              # the helpers the Mac app bundles (bridge, edit-ledger, project-files), release mode
apps/mac/scripts/make-app.sh --install --open            # packages FlashTeX.app into ~/Applications
```

`make-app.sh` bundles the helpers in `Contents/MacOS` and the engine hosts
in `Contents/Helpers` (it does not bundle `flashtex`; see
[packaging](../../apps/mac/docs/packaging.md)), and ad-hoc signs the bundle;
add `--dmg` for a disk image. `cargo test -p flashtex-build` runs the CLI's
end-to-end tests (`crates/flashtex-build/tests/cli.rs`, which compare its
PDFs with pdflatex's and so need TeX Live); `swift test` in `apps/mac` runs
the app's tests.

## Supported LaTeX

The engine is pdfTeX (Classic) or XeTeX (Unicode mode), translated from
their WEB sources, running TeX Live's own formats and packages, so a
document compiles as it does under pdflatex or xelatex with the same TeX
Live; the verification tiers (DESIGN.md §8) measure that. There is no list of
commands to check against. The Mac and iPad editors' completion vocabulary
(`supported-latex.json`) is generated from the engine's measured package
support (`apps/mac/scripts/gen-supported-latex.py`, RQ5).

## Troubleshooting

| What you see | What to do |
|---|---|
| `flashtex-host not found (--host PATH, $FLASHTEX_HOST, beside flashtex, or on PATH)` | Keep the tarball's `bin/` together, or point `--host` / `FLASHTEX_HOST` at the host |
| `flashtex-host-unicode not found …` in Unicode mode | Build or install Unicode mode's host (above), or set `FLASHTEX_HOST_UNICODE` |
| No TeX Live and no bundle | Install TeX Live (MacTeX on macOS), or set `FLASHTEX_BUNDLE_ALLOW_FETCH=1` to download the pinned bundle |
| `file:line:col: error: …` from `check` | TeX's own error, as pdflatex would report it; the `.log` is not kept, so `check` is the way to read them |
| A warning that LaTeX asks for another run | `build` ran its pass limit and cross-references may still be off; run `build` again |
