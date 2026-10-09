#!/usr/bin/env bash
# Packages the command-line tools as flashtex-cli-<version>-<platform>.tar.gz:
#
#   flashtex-cli-<version>-<platform>/
#     README.md
#     bin/flashtex               the CLI (build/check/watch/supported/worker/fonts)
#     bin/flashtex-render        (+ flashtex-compiler, flashtex-pdf,
#                                 flashtex-pdf-exact when they were built)
#     bin/flashtex-v3            the CLI on the new engine (crates/flashtex-build,
#                                 MIT; build/check/watch through the host)
#     bin/flashtex-host          the engine host it runs as a separate process
#                                 (crates/flashtex-engine, GPL-2.0-or-later)
#     bin/pdftex.pool            the engine's string pool, beside the host
#     share/flashtex/engine/LICENSE   the host's licence (GPL-2.0)
#     share/flashtex/Fonts/      pinned Latin Modern OTFs + GUST licence
#     share/flashtex/texmf/      rooted TFM metrics + licence (LM 2.004)
#     bin/Fonts -> ../share/flashtex/Fonts    (relative symlinks: the older
#     bin/texmf -> ../share/flashtex/texmf     `<exe>/Fonts` discovery)
#
# `flashtex` discovers `<exe>/../share/flashtex/{Fonts,texmf}` on its own and
# every helper still finds `<exe>/Fonts` and `<exe>/texmf` through the links
# (crates/render-pipeline/src/fonts.rs, Discovery), so the tarball needs no
# host TeX installation and no --font-dir. Binaries that do not exist are
# listed as missing in README.md instead of failing the packaging, so a
# platform where a crate does not build still ships what it has.
#
# Usage: scripts/ci/package-cli.sh <version> <platform> <out-dir>
#          [--bin <path>]... [--require <name>[,<name>...]]
#          [--fonts-dir <dir>] [--texmf-root <dir>]
#   <version>    e.g. 0.2.0 (a leading v is dropped)
#   <platform>   e.g. macos-arm64, linux-x86_64
#   <out-dir>    where the .tar.gz is written (created)
#   --bin        a built binary to include (default: flashtex, the four
#                helpers, flashtex-v3 and flashtex-host from each crate's
#                target dir (scripts/crate-target-dir.sh) when present)
#   --require    binaries that must be included: a missing one fails the
#                packaging instead of being listed as missing (a release
#                passes flashtex-v3,flashtex-host; app-parity row D5)
#   --fonts-dir  flat directory of .otf faces + GUST-FONT-LICENSE.TXT
#                (default: apps/mac/Fonts, the pinned vendored set)
#   --texmf-root rooted texmf tree with fonts/tfm/public/lm and
#                doc/fonts/lm (default: apps/mac/Fonts/texmf)
# Prints the tarball path on stdout.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
# Where the shipped GPL host's source is published (its README note).
SOURCE_URL="https://github.com/flash-tex/flashtex"

VERSION="${1:?version}"; PLATFORM="${2:?platform}"; OUT_DIR="${3:?out-dir}"
shift 3
VERSION="${VERSION#v}"
BINS=()
REQUIRE=""
FONTS_DIR="$REPO_ROOT/apps/mac/Fonts"
TEXMF_ROOT="$REPO_ROOT/apps/mac/Fonts/texmf"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --bin) BINS+=("${2:?--bin needs a path}"); shift 2 ;;
    --require) REQUIRE="${2:?--require needs a list}"; shift 2 ;;
    --fonts-dir) FONTS_DIR="${2:?}"; shift 2 ;;
    --texmf-root) TEXMF_ROOT="${2:?}"; shift 2 ;;
    -h|--help) sed -n '2,43p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) echo "package-cli.sh: unknown argument: $1" >&2; exit 2 ;;
  esac
done
die() { echo "package-cli.sh: $*" >&2; exit 1; }

if [[ ${#BINS[@]} -eq 0 ]]; then
  for p in flashtex-cli/flashtex render-pipeline/flashtex-render compiler/flashtex-compiler pdf/flashtex-pdf pdf/flashtex-pdf-exact \
           flashtex-build/flashtex-v3 flashtex-engine/flashtex-host; do
    BINS+=("$("$REPO_ROOT/scripts/crate-target-dir.sh" "$REPO_ROOT/crates/${p%%/*}")/release/${p##*/}")
  done
fi
[[ -d "$FONTS_DIR" ]] || die "fonts dir not found: $FONTS_DIR"
[[ -d "$TEXMF_ROOT/fonts/tfm/public/lm" ]] || die "texmf root has no fonts/tfm/public/lm: $TEXMF_ROOT"
[[ -f "$TEXMF_ROOT/doc/fonts/lm/GUST-FONT-LICENSE.TXT" ]] || die "texmf root has no GUST licence: $TEXMF_ROOT"

NAME="flashtex-cli-$VERSION-$PLATFORM"
STAGE="$(mktemp -d "${TMPDIR:-/tmp}/flashtex-cli.XXXXXX")"
trap 'rm -rf "$STAGE"' EXIT
ROOT="$STAGE/$NAME"
mkdir -p "$ROOT/bin" "$ROOT/share/flashtex/Fonts" "$ROOT/share/flashtex/texmf"

INCLUDED=(); MISSING=()
for b in "${BINS[@]}"; do
  if [[ -x "$b" ]]; then
    cp "$b" "$ROOT/bin/"
    INCLUDED+=("$(basename "$b")")
  else
    MISSING+=("$(basename "$b")")
  fi
done
[[ ${#INCLUDED[@]} -gt 0 ]] || die "no binary to package (looked for: ${BINS[*]})"
if [[ -n "$REQUIRE" ]]; then
  IFS=',' read -ra required <<< "$REQUIRE"
  for r in "${required[@]}"; do
    [[ " ${INCLUDED[*]} " == *" $r "* ]] || die "required binary $r was not built (looked for: ${BINS[*]})"
  done
fi
# The engine host (GPL-2.0-or-later, a separate program: DESIGN.md §3) goes
# with its string pool beside it, where flashtex-v3 and the host look for it,
# and its licence, as make-app.sh stages them in the app.
HOST_INCLUDED=0
[[ " ${INCLUDED[*]} " == *" flashtex-host "* ]] && HOST_INCLUDED=1
if [[ "$HOST_INCLUDED" -eq 1 ]]; then
  ENGINE_CRATE="$REPO_ROOT/crates/flashtex-engine"
  [[ -f "$ENGINE_CRATE/pdftex.pool" ]] || die "flashtex-host is packaged but $ENGINE_CRATE/pdftex.pool is missing"
  [[ -f "$ENGINE_CRATE/LICENSE" ]] || die "flashtex-host is packaged but its licence $ENGINE_CRATE/LICENSE is missing"
  cp "$ENGINE_CRATE/pdftex.pool" "$ROOT/bin/pdftex.pool"
  mkdir -p "$ROOT/share/flashtex/engine"
  cp "$ENGINE_CRATE/LICENSE" "$ROOT/share/flashtex/engine/LICENSE"
fi
[[ " ${INCLUDED[*]} " == *" flashtex "* ]] || echo "package-cli.sh: warning: flashtex (the CLI) is not among the binaries" >&2
[[ " ${INCLUDED[*]} " == *" flashtex-render "* ]] || echo "package-cli.sh: warning: flashtex-render is not among the binaries" >&2

# Flat faces + licence (the layout Discovery calls "flat"): only the pinned
# .otf files and the licence, never a stray file from the directory.
cp "$FONTS_DIR"/*.otf "$ROOT/share/flashtex/Fonts/"
cp "$FONTS_DIR"/GUST-FONT-LICENSE.TXT "$ROOT/share/flashtex/Fonts/"
[[ -f "$FONTS_DIR/TEX-GYRE-GUST-FONT-LICENSE.TXT" ]] && cp "$FONTS_DIR/TEX-GYRE-GUST-FONT-LICENSE.TXT" "$ROOT/share/flashtex/Fonts/"
[[ -f "$FONTS_DIR/SUPPLEMENTARY-FACES.json" ]] && cp "$FONTS_DIR/SUPPLEMENTARY-FACES.json" "$ROOT/share/flashtex/Fonts/"
# Rooted metrics tree, as vendored (TFMs + licence + pin manifest).
cp -R "$TEXMF_ROOT/." "$ROOT/share/flashtex/texmf/"
# The helpers' older `<exe>/Fonts` + `<exe>/texmf` discovery, as relative
# links so the tarball can be moved as a whole.
ln -s ../share/flashtex/Fonts "$ROOT/bin/Fonts"
ln -s ../share/flashtex/texmf "$ROOT/bin/texmf"

{
  echo "# FlashTeX command-line tools $VERSION ($PLATFORM)"
  echo
  echo "Built from https://github.com/flash-tex/flashtex (release v$VERSION)."
  echo
  echo "## Contents"
  echo
  for b in ${INCLUDED[@]+"${INCLUDED[@]}"}; do echo "- \`bin/$b\`"; done
  for b in ${MISSING[@]+"${MISSING[@]}"}; do echo "- \`bin/$b\` — not built for $PLATFORM in this release"; done
  if [[ "$HOST_INCLUDED" -eq 1 ]]; then
    echo "- \`bin/pdftex.pool\` — the engine's string pool, read by \`flashtex-host\`"
    echo "- \`share/flashtex/engine/LICENSE\` — \`flashtex-host\`'s licence (GNU GPL version 2 or later)"
  fi
  echo "- \`share/flashtex/Fonts/\` — Latin Modern and TeX Gyre (Termes, Heros, Cursor) OpenType faces (GUST Font License, see GUST-FONT-LICENSE.TXT and TEX-GYRE-GUST-FONT-LICENSE.TXT)"
  echo "- \`share/flashtex/texmf/\` — the pinned Latin Modern 2.004 TFM metrics the engine lays text out with"
  echo "- \`bin/Fonts\`, \`bin/texmf\` — links to the above for the helper binaries"
  echo
  echo "## Usage"
  echo
  echo '```'
  echo "bin/flashtex build main.tex                     # main.pdf next to it; \\input/\\include resolved from its directory"
  echo "bin/flashtex build main.tex -o out.pdf --timing # exact-route PDF (embedded font subsets, images, links)"
  echo "bin/flashtex check main.tex --json              # diagnostics only, flashtex-check/1 on stdout"
  echo "bin/flashtex watch main.tex                     # rebuild on every change; Ctrl-C stops"
  echo "bin/flashtex supported                          # implemented-LaTeX inventory + coverage"
  echo "bin/flashtex fonts                              # what this binary resolves"
  echo "bin/flashtex install-cli                        # symlink into /usr/local/bin"
  echo "bin/flashtex worker                             # runtime-v1 JSON Lines worker (what the IDE speaks)"
  echo "bin/flashtex --help"
  echo '```'
  echo
  echo "Exit status: 0 when the document rendered (ok/recovered; \`--strict\` makes recovered errors exit 1), 1 when it failed, 2 for a usage error."
  echo
  echo "The fonts and metrics are found relative to the executable (\`share/flashtex\`);"
  echo "keep the directory layout when moving the tools (\`install-cli\` links, it does not copy), or point"
  echo "\`FLASHTEX_FONT_DIRS\` / \`FLASHTEX_TFM_DIRS\` (colon separated) at your own copies. No TeX installation is required."
  echo "Reference: docs/user/compiler.md in the repository."
  if [[ " ${INCLUDED[*]} " == *" flashtex-v3 "* ]]; then
    echo
    echo "## The new engine: \`flashtex-v3\`"
    echo
    echo "\`flashtex-v3\` is the command line on the pdfLaTeX-compatible engine the app's preview uses. It"
    echo "will become \`flashtex\`; until then both ship. It needs a TeX Live installation (pdflatex's"
    echo "\`pdflatex.ini\` through \`kpsewhich\`) and runs \`flashtex-host\` from beside itself (\`--host PATH\`"
    echo "or \`FLASHTEX_HOST\` override that)."
    echo
    echo '```'
    echo "bin/flashtex-v3 build main.tex -o out.pdf        # the PDF pdflatex writes; bibtex/biber/makeindex as latexmk would run them"
    echo "bin/flashtex-v3 check main.tex --json            # TeX's diagnostics; exit 1 on an error"
    echo "bin/flashtex-v3 watch main.tex                   # rebuild on every change; Ctrl-C stops"
    echo '```'
    if [[ "$HOST_INCLUDED" -eq 1 ]]; then
      echo
      echo "\`flashtex-host\` is free software under the GNU General Public License, version 2 or later"
      echo "(\`share/flashtex/engine/LICENSE\`): a translation of pdfTeX. Its source is crates/flashtex-engine"
      echo "in $SOURCE_URL at tag v$VERSION ($SOURCE_URL/tree/v$VERSION/crates/flashtex-engine)."
      echo "\`flashtex-v3\` (MIT) does not link it; it runs it as a separate program."
    fi
  fi
} > "$ROOT/README.md"

mkdir -p "$OUT_DIR"
OUT="$(cd "$OUT_DIR" && pwd)/$NAME.tar.gz"
tar -C "$STAGE" -czf "$OUT" "$NAME"
echo "$OUT"
