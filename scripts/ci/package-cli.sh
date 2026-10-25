#!/usr/bin/env bash
# Packages the command line as flashtex-cli-<version>-<platform>.tar.gz:
#
#   flashtex-cli-<version>-<platform>/
#     README.md
#     SOURCE.md                  the written offer of the hosts' GPL source
#     bin/flashtex               the CLI (crates/flashtex-build, MIT;
#                                 build/check/watch through an engine host)
#     bin/flashtex-host          Classic mode's engine host, run as a separate
#                                 program (crates/flashtex-engine, GPL-2.0-or-later)
#     bin/flashtex-host-unicode  Unicode mode's engine host, when it was built
#                                 (crates/flashtex-xetex, GPL-2.0-or-later)
#     bin/pdftex.pool            flashtex-host's string pool, beside it
#     bin/flashtex-bundle.lock   the pinned no-TeX-Live bundle the hosts may
#                                 fetch with the user's consent
#                                 (tools/bundle/tl2026, as the app ships it)
#     share/flashtex/LICENSE     the CLI's licence (MIT)
#     share/flashtex/engine/LICENSE, THIRD-PARTY-NOTICES
#                                flashtex-host's licence (GPL-2.0) and the
#                                 notices of what it links (jemalloc, on Linux)
#     share/flashtex/engine/LICENSE-unicode, THIRD-PARTY-NOTICES-unicode.txt
#                                the same for flashtex-host-unicode (HarfBuzz,
#                                 FreeType, TECkit, ICU), when it ships
#
# `flashtex` finds the host beside itself and the host finds its pool and
# the bundle lock beside itself, so the directory can be moved as a whole.
# The old engine's CLI, its helpers (flashtex-render, flashtex-compiler,
# flashtex-pdf, flashtex-pdf-exact) and the Latin Modern fonts and metrics
# they needed left with that engine (retirement plan S7, RQ6). Binaries that
# do not exist are listed as missing in README.md instead of failing the
# packaging, so a platform where a crate does not build still ships what it
# has, unless --require names them.
#
# Usage: scripts/ci/package-cli.sh <version> <platform> <out-dir>
#          [--bin <path>]... [--require <name>[,<name>...]]
#   <version>    e.g. 0.2.0 (a leading v is dropped)
#   <platform>   e.g. macos-arm64, linux-x86_64
#   <out-dir>    where the .tar.gz is written (created)
#   --bin        a built binary to include (default: flashtex, flashtex-host
#                and flashtex-host-unicode from each crate's target dir,
#                scripts/crate-target-dir.sh, when present)
#   --require    binaries that must be included: a missing one fails the
#                packaging instead of being listed as missing (a release
#                passes flashtex,flashtex-host; app-parity row D5)
# Prints the tarball path on stdout.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
# Where the shipped GPL hosts' source is published (SOURCE.md, README.md).
SOURCE_URL="https://github.com/flash-tex/flashtex"
# The app's About panel and the release notes carry the same line
# (apps/mac/Sources/FlashTeXMac/AboutPanel.swift, crates/flashtex-build/src/licenses.rs).
SOURCE_LINE="Source code for the FlashTeX engine (GPL-2.0-or-later): https://github.com/flash-tex/flashtex"

VERSION="${1:?version}"; PLATFORM="${2:?platform}"; OUT_DIR="${3:?out-dir}"
shift 3
VERSION="${VERSION#v}"
BINS=()
REQUIRE=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --bin) BINS+=("${2:?--bin needs a path}"); shift 2 ;;
    --require) REQUIRE="${2:?--require needs a list}"; shift 2 ;;
    -h|--help) sed -n '2,44p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) echo "package-cli.sh: unknown argument: $1" >&2; exit 2 ;;
  esac
done
die() { echo "package-cli.sh: $*" >&2; exit 1; }

if [[ ${#BINS[@]} -eq 0 ]]; then
  for p in flashtex-build/flashtex flashtex-engine/flashtex-host flashtex-xetex/flashtex-host-unicode; do
    BINS+=("$("$REPO_ROOT/scripts/crate-target-dir.sh" "$REPO_ROOT/crates/${p%%/*}")/release/${p##*/}")
  done
fi

NAME="flashtex-cli-$VERSION-$PLATFORM"
STAGE="$(mktemp -d "${TMPDIR:-/tmp}/flashtex-cli.XXXXXX")"
trap 'rm -rf "$STAGE"' EXIT
ROOT="$STAGE/$NAME"
mkdir -p "$ROOT/bin" "$ROOT/share/flashtex"

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
has() { [[ " ${INCLUDED[*]} " == *" $1 "* ]]; }
has flashtex || echo "package-cli.sh: warning: flashtex (the CLI) is not among the binaries" >&2
cp "$REPO_ROOT/LICENSE" "$ROOT/share/flashtex/LICENSE"

# The engine hosts (GPL-2.0-or-later, separate programs: DESIGN.md §3) go
# with what they read beside them and their licences and notices, as
# apps/mac/scripts/make-app.sh stages them in the app.
BUNDLE_LOCK="$REPO_ROOT/tools/bundle/tl2026/flashtex-bundle.lock"
HOSTS=0
if has flashtex-host; then
  HOSTS=1
  ENGINE_CRATE="$REPO_ROOT/crates/flashtex-engine"
  for f in pdftex.pool LICENSE THIRD-PARTY-NOTICES; do
    [[ -f "$ENGINE_CRATE/$f" ]] || die "flashtex-host is packaged but $ENGINE_CRATE/$f is missing"
  done
  cp "$ENGINE_CRATE/pdftex.pool" "$ROOT/bin/pdftex.pool"
  mkdir -p "$ROOT/share/flashtex/engine"
  cp "$ENGINE_CRATE/LICENSE" "$ROOT/share/flashtex/engine/LICENSE"
  cp "$ENGINE_CRATE/THIRD-PARTY-NOTICES" "$ROOT/share/flashtex/engine/THIRD-PARTY-NOTICES"
fi
if has flashtex-host-unicode; then
  HOSTS=1
  XETEX_CRATE="$REPO_ROOT/crates/flashtex-xetex"
  HB="$REPO_ROOT/third_party/harfbuzz/harfbuzz-src/COPYING"
  FT="$REPO_ROOT/third_party/freetype/freetype-src/LICENSE.TXT"
  FTL="$REPO_ROOT/third_party/freetype/freetype-src/docs/FTL.TXT"
  TK="$REPO_ROOT/third_party/teckit/TECkit-src/license/License_LGPLv21.txt"
  ICU="$REPO_ROOT/third_party/icu/LICENSE"
  for f in "$XETEX_CRATE/LICENSE" "$HB" "$FT" "$FTL" "$TK" "$ICU"; do
    [[ -f "$f" ]] || die "flashtex-host-unicode is packaged but its licence or a notice is missing: $f"
  done
  mkdir -p "$ROOT/share/flashtex/engine"
  cp "$XETEX_CRATE/LICENSE" "$ROOT/share/flashtex/engine/LICENSE-unicode"
  {
    echo "flashtex-host-unicode links these libraries, vendored unmodified (third_party/):"
    echo
    echo "== HarfBuzz (MIT-style; third_party/harfbuzz/harfbuzz-src/COPYING) =="
    cat "$HB"
    echo
    echo "== FreeType (used under the FreeType License, docs/FTL.TXT; portions of this software are copyright (C) The FreeType Project, www.freetype.org) =="
    cat "$FT"
    echo
    cat "$FTL"
    echo
    echo "== TECkit (used under the GNU LGPL 2.1 or later; third_party/teckit/TECkit-src/license) =="
    cat "$TK"
    echo
    echo "== ICU's line-break tables and bidi algorithm (third_party/icu) and ICU4X's segmenter and its data (Unicode License v3) =="
    cat "$ICU"
  } > "$ROOT/share/flashtex/engine/THIRD-PARTY-NOTICES-unicode.txt"
fi
if [[ "$HOSTS" -eq 1 ]]; then
  [[ -f "$BUNDLE_LOCK" ]] || die "an engine host is packaged but $BUNDLE_LOCK (the pinned no-TeX-Live bundle) is missing"
  cp "$BUNDLE_LOCK" "$ROOT/bin/flashtex-bundle.lock"
fi

# The written source offer (retirement plan §6, RQ19): what `flashtex
# licenses` prints, as a file that travels with the binaries.
{
  echo "# Source code"
  echo
  echo "$SOURCE_LINE"
  echo
  echo "This release: $SOURCE_URL/tree/v$VERSION. The release also attaches the"
  echo "source of that tag as \`flashtex-$VERSION-source.tar.gz\`."
  echo
  echo "- \`bin/flashtex\` is free software under the MIT License (\`share/flashtex/LICENSE\`);"
  echo "  its source is crates/flashtex-build. It holds no engine code and runs an engine"
  echo "  host as a separate program."
  echo "- \`bin/flashtex-host\` (crates/flashtex-engine, a translation of pdfTeX) and"
  echo "  \`bin/flashtex-host-unicode\` (crates/flashtex-xetex, a translation of XeTeX) are free"
  echo "  software under the GNU General Public License, version 2 or later"
  echo "  (\`share/flashtex/engine/\`). They link TeX Live 2026's kpathsea, zlib, libpng and"
  echo "  xpdf (xpdf is GPL version 2 or 3 only, so the hosts are distributed under GPL"
  echo "  version 2 or 3), and flashtex-host-unicode also HarfBuzz, FreeType, TECkit and"
  echo "  ICU's data. These are vendored under third_party/ at the same tag, each with a"
  echo "  README that names its version and upstream pin."
  echo
  echo "You may have the complete corresponding source of all of it from the addresses"
  echo "above, for as long as this release is offered."
} > "$ROOT/SOURCE.md"

{
  echo "# FlashTeX command line $VERSION ($PLATFORM)"
  echo
  echo "Built from $SOURCE_URL (release v$VERSION)."
  echo
  echo "## Contents"
  echo
  for b in ${INCLUDED[@]+"${INCLUDED[@]}"}; do echo "- \`bin/$b\`"; done
  for b in ${MISSING[@]+"${MISSING[@]}"}; do echo "- \`bin/$b\` — not built for $PLATFORM in this release"; done
  if has flashtex-host; then
    echo "- \`bin/pdftex.pool\` — the engine's string pool, read by \`flashtex-host\`"
  fi
  if [[ "$HOSTS" -eq 1 ]]; then
    echo "- \`bin/flashtex-bundle.lock\` — the pinned TeX bundle a host may download when there is no TeX Live (only with \`FLASHTEX_BUNDLE_ALLOW_FETCH=1\`)"
    echo "- \`share/flashtex/engine/\` — the engine hosts' licences (GNU GPL version 2 or later) and the notices of the code they are built with"
  fi
  echo "- \`share/flashtex/LICENSE\` — \`flashtex\`'s licence (MIT)"
  echo "- \`SOURCE.md\` — where the source of all of it is"
  echo
  echo "## Usage"
  echo
  echo '```'
  echo "bin/flashtex build main.tex                 # main.pdf next to it: the PDF pdflatex writes (bibtex/biber/makeindex as latexmk would run them)"
  echo "bin/flashtex build main.tex --mode unicode  # Unicode mode (XeTeX): fontspec, unicode-math, system fonts"
  echo "bin/flashtex check main.tex --json          # TeX's diagnostics; exit 1 on an error"
  echo "bin/flashtex watch main.tex                 # rebuild on every change, one warm engine; Ctrl-C stops"
  echo "bin/flashtex licenses                       # the licences and the source offer"
  echo "bin/flashtex --help"
  echo '```'
  echo
  echo "\`flashtex\` needs a TeX Live installation (pdflatex's \`pdflatex.ini\` through \`kpsewhich\`), or"
  echo "the pinned bundle (\`FLASHTEX_BUNDLE_ALLOW_FETCH=1\`). It runs the engine host from beside itself"
  echo "(\`--host PATH\` or \`FLASHTEX_HOST\` override that); keep the directory together when moving it."
  echo "Reference: docs/user/compiler.md in the repository."
  if [[ "$HOSTS" -eq 1 ]]; then
    echo
    echo "The engine hosts are free software under the GNU General Public License, version 2 or later"
    echo "(\`share/flashtex/engine/LICENSE\`): translations of pdfTeX and XeTeX. Their source is"
    echo "crates/flashtex-engine and crates/flashtex-xetex in $SOURCE_URL at tag v$VERSION"
    echo "($SOURCE_URL/tree/v$VERSION/crates/flashtex-engine); see SOURCE.md."
    echo "\`flashtex\` (MIT) does not link them; it runs them as separate programs."
  fi
} > "$ROOT/README.md"

mkdir -p "$OUT_DIR"
OUT="$(cd "$OUT_DIR" && pwd)/$NAME.tar.gz"
tar -C "$STAGE" -czf "$OUT" "$NAME"
echo "$OUT"
