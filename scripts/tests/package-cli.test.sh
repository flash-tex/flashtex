#!/usr/bin/env bash
# Self-test for scripts/ci/package-cli.sh and the release workflow's use of
# it (app-parity row D5, RQ6): the CLI tarball ships `flashtex`
# (crates/flashtex-build) and the engine hosts, with the host's string pool,
# the bundle lock, the licences and the written source offer; a release that
# requires them fails without them; nothing of the old engine's CLI ships;
# and .github/workflows/release.yml builds them, requires them in the
# tarball, requires the host in the app (make-app.sh --require-engine-host)
# and attaches the source. No cargo: the binaries are stand-in scripts.
#
# Usage: scripts/tests/package-cli.test.sh   (exit 1 on a failure)
# Portable: bash 3.2 (macOS) and Linux. Run by apps/mac/scripts/packaging-selftest.sh
# and by `scripts/gate.sh pr`.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PKG="$ROOT/scripts/ci/package-cli.sh"
REL="$ROOT/.github/workflows/release.yml"
MAKE="$ROOT/apps/mac/scripts/make-app.sh"
ENGINE="$ROOT/crates/flashtex-engine"
XETEX="$ROOT/crates/flashtex-xetex"
LOCK="$ROOT/tools/bundle/tl2026/flashtex-bundle.lock"
SOURCE_LINE="Source code for the FlashTeX engine (GPL-2.0-or-later): https://github.com/flash-tex/flashtex"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/package-cli-test.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

FAIL=0
ok() { echo "  ok   $1"; }
bad() { echo "  FAIL $1" >&2; FAIL=$((FAIL + 1)); }
check() { # $1=description, then the command
  local what="$1"; shift
  if "$@"; then ok "$what"; else bad "$what"; fi
}

mkdir -p "$WORK/bins"
for b in flashtex flashtex-host flashtex-host-unicode; do
  printf '#!/bin/sh\necho %s\n' "$b" > "$WORK/bins/$b"
  chmod +x "$WORK/bins/$b"
done
ALL=()
for b in flashtex flashtex-host flashtex-host-unicode; do ALL+=(--bin "$WORK/bins/$b"); done

echo "==> package-cli.sh: the CLI and both hosts, required"
if tarball="$("$PKG" 9.9.9 selftest "$WORK/out1" "${ALL[@]}" --require flashtex,flashtex-host 2>"$WORK/err1")"; then
  mkdir -p "$WORK/x1"
  tar -C "$WORK/x1" -xzf "$tarball"
  d="$WORK/x1/flashtex-cli-9.9.9-selftest"
  for f in bin/flashtex bin/flashtex-host bin/flashtex-host-unicode; do
    check "$f is in the tarball, executable" test -x "$d/$f"
  done
  for f in bin/flashtex-render bin/flashtex-compiler bin/flashtex-pdf bin/flashtex-pdf-exact bin/flashtex-v3 bin/Fonts bin/texmf share/flashtex/Fonts share/flashtex/texmf; do
    check "nothing of the old CLI: no $f" test ! -e "$d/$f"
  done
  check "bin/pdftex.pool is the engine's pool, beside the host" cmp -s "$d/bin/pdftex.pool" "$ENGINE/pdftex.pool"
  check "bin/flashtex-bundle.lock is the pinned bundle's lock" cmp -s "$d/bin/flashtex-bundle.lock" "$LOCK"
  check "share/flashtex/LICENSE is the CLI's (MIT)" cmp -s "$d/share/flashtex/LICENSE" "$ROOT/LICENSE"
  check "share/flashtex/engine/LICENSE is the engine's licence" cmp -s "$d/share/flashtex/engine/LICENSE" "$ENGINE/LICENSE"
  check "share/flashtex/engine/THIRD-PARTY-NOTICES is the engine's notices" cmp -s "$d/share/flashtex/engine/THIRD-PARTY-NOTICES" "$ENGINE/THIRD-PARTY-NOTICES"
  check "the notices carry jemalloc's" grep -q 'Copyright (C) 2002-present Jason Evans' "$d/share/flashtex/engine/THIRD-PARTY-NOTICES"
  check "share/flashtex/engine/LICENSE-unicode is the Unicode host's licence" cmp -s "$d/share/flashtex/engine/LICENSE-unicode" "$XETEX/LICENSE"
  check "the Unicode host's notices name HarfBuzz, FreeType, TECkit and ICU" bash -c 'for n in HarfBuzz FreeType TECkit ICU; do grep -q "== $n" "$1" || exit 1; done' _ "$d/share/flashtex/engine/THIRD-PARTY-NOTICES-unicode.txt"
  check "SOURCE.md carries the About panel's source line" grep -qF "$SOURCE_LINE" "$d/SOURCE.md"
  check "SOURCE.md names this release's tree and source archive" bash -c 'grep -qF "https://github.com/flash-tex/flashtex/tree/v9.9.9" "$1" && grep -qF "flashtex-9.9.9-source.tar.gz" "$1"' _ "$d/SOURCE.md"
  check "SOURCE.md names the linked libraries" bash -c 'for n in kpathsea zlib libpng xpdf HarfBuzz FreeType TECkit; do grep -q "$n" "$1" || exit 1; done' _ "$d/SOURCE.md"
  check "README lists bin/flashtex and bin/flashtex-host" bash -c 'grep -q "bin/flashtex\`$" "$1" && grep -q "bin/flashtex-host\`$" "$1"' _ "$d/README.md"
  check "README shows flashtex's usage" grep -q '^bin/flashtex build main.tex' "$d/README.md"
  check "README shows flashtex licenses" grep -q '^bin/flashtex licenses' "$d/README.md"
  check "README states the hosts' licence" grep -q 'GNU General Public License, version 2 or later' "$d/README.md"
  check "README names where the host's source is" grep -qF 'https://github.com/flash-tex/flashtex/tree/v9.9.9/crates/flashtex-engine' "$d/README.md"
  check "README's code fences are plain (no backslash)" bash -c '! grep -qF "\\\`" "$1"' _ "$d/README.md"
else
  bad "package-cli.sh with every binary and --require failed: $(head -3 "$WORK/err1" | tr '\n' ' ')"
fi

echo "==> package-cli.sh: a required host that was not built"
rc=0
"$PKG" 9.9.9 selftest "$WORK/out2" --bin "$WORK/bins/flashtex" --bin "$WORK/missing/flashtex-host" \
  --require flashtex,flashtex-host >/dev/null 2>"$WORK/err2" || rc=$?
check "exits 1" test "$rc" -eq 1
check "names the missing binary" grep -q 'required binary flashtex-host was not built' "$WORK/err2"
check "writes no tarball" test ! -e "$WORK/out2/flashtex-cli-9.9.9-selftest.tar.gz"

echo "==> package-cli.sh: without --require a missing host is listed, as before"
if tarball="$("$PKG" 9.9.9 selftest "$WORK/out3" --bin "$WORK/bins/flashtex" --bin "$WORK/missing/flashtex-host" 2>"$WORK/err3")"; then
  mkdir -p "$WORK/x3"
  tar -C "$WORK/x3" -xzf "$tarball"
  d="$WORK/x3/flashtex-cli-9.9.9-selftest"
  check "README lists flashtex-host as not built" grep -q 'bin/flashtex-host` — not built for selftest' "$d/README.md"
  check "no pool without the host" test ! -e "$d/bin/pdftex.pool"
  check "no bundle lock without a host" test ! -e "$d/bin/flashtex-bundle.lock"
  check "no engine licence without the host" test ! -e "$d/share/flashtex/engine"
  check "the source offer ships anyway" grep -qF "$SOURCE_LINE" "$d/SOURCE.md"
else
  bad "package-cli.sh without --require failed: $(head -3 "$WORK/err3" | tr '\n' ' ')"
fi

echo "==> package-cli.sh: the old packaging options are gone"
rc=0
"$PKG" 9.9.9 selftest "$WORK/out4" --bin "$WORK/bins/flashtex" --fonts-dir "$WORK" >/dev/null 2>"$WORK/err4" || rc=$?
check "--fonts-dir is refused (exit 2)" test "$rc" -eq 2

echo "==> release.yml ships the CLI, the hosts and the source"
check "builds flashtex-host" grep -q -- '-p flashtex-engine --bin flashtex-host' "$REL"
check "builds flashtex" grep -q -- '-p flashtex-build --bin flashtex' "$REL"
check "builds nothing of the old engine" bash -c '! grep -nE -- "-p flashtex-(cli|render-pipeline|compiler|pdf)( |$)|flashtex-v3|flashtex-render" "$1"' _ "$REL"
check "requires the host in the app (make-app.sh --require-engine-host)" grep -q -- 'args=(--version "$VERSION" --dmg --require-engine-host)' "$REL"
check "requires flashtex and flashtex-host in the macOS CLI tarball" grep -q -- '--require flashtex,flashtex-host' "$REL"
check "make-app.sh takes --require-engine-host" grep -q -- '--require-engine-host)' "$MAKE"
check "smoke steps never pipe a program into grep -q (SIGPIPE under pipefail)" bash -c '! grep -nE -- "(--help|--version|licenses) \| grep -q" "$1"' _ "$REL"
check "the notes carry the About panel's source line" grep -qF "$SOURCE_LINE" "$REL"
check "the release attaches its source archive" grep -q -- 'flashtex-$VERSION-source.tar.gz' "$REL"
check "a dry run publishes nothing" grep -q -- "if: \${{ !inputs.dry_run }}" "$REL"

if [[ "$FAIL" -gt 0 ]]; then
  echo "package-cli self-test: $FAIL failure(s)" >&2
  exit 1
fi
echo "package-cli self-test: all passed"
